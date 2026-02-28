use crate::input_and_compile_error;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{ItemFn, LitInt, LitStr, Token};

macro_rules! job_args_parse {
    (
        $($name:ident, $trigger_type:ident, $lower:ident, $trigger_runtime_type:ty,)+
    ) => {
        pub(crate) enum JobType {
            $(
                $name,
            )+
        }

        impl JobType {
            fn parse_args(self, args: TokenStream) -> syn::Result<JobArgs> {
                match self {
                    $(
                        Self::$name => syn::parse::<$name>(args).map(JobArgs::from),
                    )+
                }
            }
        }

        pub enum JobArgs {
            $(
                $name($trigger_type),
            )+
        }

        $(
            #[derive(Debug, Clone, PartialEq, Eq, Hash)]
            struct $name($trigger_type);

            impl syn::parse::Parse for $name {
                fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
                    let trigger = input.parse::<syn::$trigger_type>().map_err(|mut err| {
                        err.combine(syn::Error::new(
                            err.span(),
                            concat!("invalid job definition, expected #[", stringify!($lower), "(", stringify!($trigger_runtime_type), ")]"),
                        ));
                        err
                    })?;

                    // if there's no comma, assume that no options are provided
                    if input.peek(Token![,]) {
                        return Err(syn::Error::new(
                            Span::call_site(),
                            "Unknown attribute key is specified",
                        ));
                    }
                    Ok($name(trigger))
                }
            }

            impl From<$name> for JobArgs {
                fn from($lower: $name) -> Self {
                    Self::$name($lower.0)
                }
            }
        )+
    };
}

#[rustfmt::skip]
job_args_parse!(
    OneShot, LitInt, one_shot, u64, 
    FixDelay, LitInt, fix_delay, u64, 
    FixRate, LitInt, fix_rate, u64, 
    Cron, LitStr, cron, str,
);

pub(crate) struct Job {
    /// Name of the handler function being annotated.
    name: syn::Ident,

    /// Args passed to job macro.
    args: JobArgs,

    /// AST of the handler function being annotated.
    ast: syn::ItemFn,

    /// The doc comment attributes to copy to generated struct, if any.
    doc_attributes: Vec<syn::Attribute>,
}

impl Job {
    fn new(args: JobArgs, ast: ItemFn) -> syn::Result<Self> {
        let name = ast.sig.ident.clone();

        // Try and pull out the doc comments so that we can reapply them to the generated struct.
        // Note that multi line doc comments are converted to multiple doc attributes.
        let doc_attributes = ast
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("doc"))
            .cloned()
            .collect();

        if ast.sig.asyncness.is_none() {
            return Err(syn::Error::new_spanned(
                ast.sig.fn_token,
                "only support async fn",
            ));
        }

        Ok(Self {
            name,
            args,
            ast,
            doc_attributes,
        })
    }
}

impl ToTokens for Job {
    fn to_tokens(&self, output: &mut TokenStream2) {
        let Self {
            name,
            ast,
            args,
            doc_attributes,
        } = self;

        #[allow(unused_variables)] // used when force-pub feature is disabled
        let vis = &ast.vis;

        let register_stream = match args {
            JobArgs::OneShot(literal) => {
                quote! { __jobs.add_job(::summer_job::job::Job::one_shot(#literal).run(#name))}
            }
            JobArgs::FixDelay(literal) => {
                quote! { __jobs.add_job(::summer_job::job::Job::fix_delay(#literal).run(#name))}
            }
            JobArgs::FixRate(literal) => {
                quote! { __jobs.add_job(::summer_job::job::Job::fix_rate(#literal).run(#name))}
            }
            JobArgs::Cron(literal) => {
                quote! { __jobs.add_job(::summer_job::job::Job::cron(#literal).run(#name))}
            }
        };
        let stream = quote! {
            #(#doc_attributes)*
            #[allow(non_camel_case_types, missing_docs)]
            #vis struct #name;

            impl ::summer_job::handler::TypedHandlerRegistrar for #name {
                fn install_job(&self, mut __jobs: ::summer_job::Jobs) -> ::summer_job::Jobs {
                    use ::summer_job::JobConfigurator;
                    use ::summer_job::job::JobBuilder;
                    #ast
                    #register_stream
                }
            }

            ::summer_job::submit_typed_handler!(#name);
        };

        output.extend(stream);
    }
}

pub(crate) fn with_job(job_type: JobType, args: TokenStream, input: TokenStream) -> TokenStream {
    let args = match job_type.parse_args(args) {
        Ok(job) => job,
        // on parse error, make IDEs happy; see fn docs
        Err(err) => return input_and_compile_error(input, err),
    };

    let ast = match syn::parse::<syn::ItemFn>(input.clone()) {
        Ok(ast) => ast,
        // on parse error, make IDEs happy; see fn docs
        Err(err) => return input_and_compile_error(input, err),
    };

    match Job::new(args, ast) {
        Ok(job) => job.into_token_stream().into(),
        // on macro related error, make IDEs happy; see fn docs
        Err(err) => input_and_compile_error(input, err),
    }
}
