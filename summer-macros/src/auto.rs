use crate::input_and_compile_error;
use proc_macro::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::{Expr, ExprCall, ExprMethodCall};
use syn::{ItemFn, Stmt, Token};

struct ConfigArgs {
    route: bool,
    job: bool,
}

impl syn::parse::Parse for ConfigArgs {
    fn parse(args: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut route = false;
        let mut job = false;

        while !args.is_empty() {
            let ident = args.parse::<syn::Ident>().map_err(|mut err| {
                err.combine(syn::Error::new(
                    err.span(),
                    r#"invalid auto config, expected #[auto_config(Configurator)]"#,
                ));

                err
            })?;
            if ident == "WebConfigurator" {
                route = true;
            }
            if ident == "JobConfigurator" {
                job = true;
            }
            if !args.peek(Token![,]) {
                break;
            }
            args.parse::<Token![,]>()?;
        }

        Ok(ConfigArgs { route, job })
    }
}

struct AppConfig {
    args: ConfigArgs,
    ast: ItemFn,
}

impl AppConfig {
    fn new(args: ConfigArgs, ast: ItemFn) -> Self {
        Self { args, ast }
    }
}

impl ToTokens for AppConfig {
    fn to_tokens(&self, output: &mut proc_macro2::TokenStream) {
        let args = &self.args;
        let mut input_fn = self.ast.clone();

        input_fn.block.stmts = input_fn
            .block
            .stmts
            .into_iter()
            .map(|stmt| process_stmt(stmt, args))
            .collect();

        // 生成输出的TokenStream
        output.extend(quote! {
            #input_fn
        });
    }
}

fn process_stmt(stmt: Stmt, args: &ConfigArgs) -> Stmt {
    match stmt {
        Stmt::Expr(expr, semi) => Stmt::Expr(process_expr(expr, args), semi),
        other => other,
    }
}

fn process_expr(expr: Expr, args: &ConfigArgs) -> Expr {
    match expr {
        // Handle method calls
        Expr::MethodCall(mut method_call) => {
            method_call.receiver = Box::new(process_expr(*method_call.receiver, args));
            Expr::MethodCall(method_call)
        }
        // Handle function calls
        Expr::Call(mut call) => {
            call.func = Box::new(process_expr(*call.func, args));

            // Clone the function for later use to avoid partial move
            if let Expr::Path(ref expr_path) = *call.func {
                if is_app_new_call(&expr_path.path) {
                    // Modify the call to add the router with parameter
                    return add_method_call(call, args);
                }
            }

            Expr::Call(call)
        }
        // Handle await expressions
        Expr::Await(mut expr_await) => {
            expr_await.base = Box::new(process_expr(*expr_await.base, args));
            Expr::Await(expr_await)
        }
        // For all other expressions, return as is
        other => other,
    }
}

fn add_method_call(call: ExprCall, args: &ConfigArgs) -> Expr {
    let mut expr = Expr::Call(call);
    if args.route {
        expr = Expr::MethodCall(ExprMethodCall {
            attrs: vec![],
            receiver: Box::new(expr),
            dot_token: Default::default(),
            method: syn::parse_quote!(add_router),
            turbofish: None,
            paren_token: Default::default(),
            args: {
                let mut punctuated = syn::punctuated::Punctuated::new();
                punctuated.push(syn::parse_quote!(::summer_web::handler::auto_router()));
                punctuated
            },
        });
    }
    if args.job {
        expr = Expr::MethodCall(ExprMethodCall {
            attrs: vec![],
            receiver: Box::new(expr),
            dot_token: Default::default(),
            method: syn::parse_quote!(add_jobs),
            turbofish: None,
            paren_token: Default::default(),
            args: {
                let mut punctuated = syn::punctuated::Punctuated::new();
                punctuated.push(syn::parse_quote!(::summer_job::handler::auto_jobs()));
                punctuated
            },
        });
    }
    expr
}

fn is_app_new_call(path: &syn::Path) -> bool {
    // Check if the path corresponds to `App::new`
    path.segments.len() == 2 && path.segments[0].ident == "App" && path.segments[1].ident == "new"
}

pub(crate) fn config(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = match syn::parse(args) {
        Ok(config) => config,
        Err(e) => return input_and_compile_error(input, e),
    };

    let ast = match syn::parse::<syn::ItemFn>(input.clone()) {
        Ok(ast) => ast,
        // on parse error, make IDEs happy; see fn docs
        Err(err) => return input_and_compile_error(input, err),
    };

    AppConfig::new(args, ast).into_token_stream().into()
}
