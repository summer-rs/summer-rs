use crate::input_and_compile_error;
use proc_macro::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::{ItemFn, LitStr, Token};

struct PubsubListenerArgs {
    subscription: LitStr,
}

impl syn::parse::Parse for PubsubListenerArgs {
    fn parse(args: syn::parse::ParseStream) -> syn::Result<Self> {
        let subscription = args.parse::<LitStr>().map_err(|mut err| {
            err.combine(syn::Error::new(
                err.span(),
                r#"invalid pubsub_listener, expected #[pubsub_listener("subscription-id")]"#,
            ));
            err
        })?;
        if args.peek(Token![,]) {
            args.parse::<Token![,]>()?;
        }
        if !args.is_empty() {
            return Err(syn::Error::new(
                args.span(),
                "pubsub_listener only accepts a single subscription literal for now",
            ));
        }
        Ok(Self { subscription })
    }
}

pub(crate) struct PubsubListener {
    name: syn::Ident,
    args: PubsubListenerArgs,
    ast: syn::ItemFn,
    doc_attributes: Vec<syn::Attribute>,
}

impl PubsubListener {
    fn new(args: PubsubListenerArgs, ast: ItemFn) -> syn::Result<Self> {
        let name = ast.sig.ident.clone();
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

impl ToTokens for PubsubListener {
    fn to_tokens(&self, output: &mut proc_macro2::TokenStream) {
        let Self {
            name,
            ast,
            args,
            doc_attributes,
        } = self;
        #[allow(unused_variables)]
        let vis = &ast.vis;
        let subscription = &args.subscription;

        let stream = quote! {
            #(#doc_attributes)*
            #[allow(non_camel_case_types, missing_docs)]
            #vis struct #name;

            impl ::summer_pubsub::handler::TypedHandlerRegistrar for #name {
                fn install_consumer(&self, mut consumers: ::summer_pubsub::Consumers) -> ::summer_pubsub::Consumers {
                    use ::summer_pubsub::PubSubConfigurator;
                    #ast

                    consumers.add_consumer(
                        ::summer_pubsub::consumer::Consumer::default()
                            .consume(#subscription, #name)
                    )
                }
            }

            ::summer_pubsub::submit_typed_handler!(#name);
        };

        output.extend(stream);
    }
}

pub(crate) fn listener(args: TokenStream, input: TokenStream) -> TokenStream {
    let args: PubsubListenerArgs = match syn::parse(args) {
        Ok(config) => config,
        Err(e) => return input_and_compile_error(input, e),
    };
    let ast = match syn::parse::<syn::ItemFn>(input.clone()) {
        Ok(ast) => ast,
        Err(err) => return input_and_compile_error(input, err),
    };
    match PubsubListener::new(args, ast) {
        Ok(job) => job.into_token_stream().into(),
        Err(err) => input_and_compile_error(input, err),
    }
}
