use crate::input_and_compile_error;
use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, LitStr};

pub(crate) fn on_connection(_args: TokenStream, input: TokenStream) -> TokenStream {
    let ast = match syn::parse::<ItemFn>(input.clone()) {
        Ok(ast) => ast,
        Err(err) => return input_and_compile_error(input, err),
    };

    let handler_name = &ast.sig.ident;
    let handler_struct_name = syn::Ident::new(
        &format!("__SocketIOConnectHandler_{handler_name}"),
        handler_name.span(),
    );

    let output = quote! {
        #ast

        #[allow(non_camel_case_types)]
        pub struct #handler_struct_name;

        impl ::summer_web::handler::SocketIOHandlerRegistrar for #handler_struct_name {
            fn install_socketio_handlers(&self, socket: &::summer_web::socketioxide::extract::SocketRef) {
                use ::summer_web::socketioxide::handler::connect::ConnectHandler;
                use ::summer_web::socketioxide::adapter::LocalAdapter;
                use std::ops::Deref;

                // SocketRef is a newtype around Arc<Socket>, we need to extract it
                let socket_clone = socket.clone();
                // SocketRef derefs to Socket, so &*socket gives us &Socket
                // We need Arc<Socket>, so we clone the Arc through the SocketRef
                let socket_arc = unsafe {
                    // SocketRef is repr(transparent) over Arc<Socket>
                    std::mem::transmute::<::summer_web::socketioxide::extract::SocketRef, std::sync::Arc<::summer_web::socketioxide::socket::Socket<LocalAdapter>>>(socket_clone)
                };

                ::summer_web::socketioxide::handler::connect::ConnectHandler::call(&#handler_name, socket_arc, None);
            }
        }

        ::summer_web::handler::submit! {
            &(#handler_struct_name) as &dyn ::summer_web::handler::SocketIOHandlerRegistrar
        }
    };

    output.into()
}

pub(crate) fn on_disconnect(_args: TokenStream, input: TokenStream) -> TokenStream {
    let ast = match syn::parse::<ItemFn>(input.clone()) {
        Ok(ast) => ast,
        Err(err) => return input_and_compile_error(input, err),
    };

    let handler_name = &ast.sig.ident;
    let handler_struct_name = syn::Ident::new(
        &format!("__SocketIODisconnectHandler_{handler_name}"),
        handler_name.span(),
    );

    let output = quote! {
        #ast

        #[allow(non_camel_case_types)]
        pub struct #handler_struct_name;

        impl ::summer_web::handler::SocketIOHandlerRegistrar for #handler_struct_name {
            fn install_socketio_handlers(&self, socket: &::summer_web::socketioxide::extract::SocketRef) {
                socket.on_disconnect(#handler_name);
            }
        }

        ::summer_web::handler::submit! {
            &(#handler_struct_name) as &dyn ::summer_web::handler::SocketIOHandlerRegistrar
        }
    };

    output.into()
}

pub(crate) fn subscribe_message(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = match syn::parse::<LitStr>(args) {
        Ok(args) => args,
        Err(err) => return input_and_compile_error(input, err),
    };

    let ast = match syn::parse::<ItemFn>(input.clone()) {
        Ok(ast) => ast,
        Err(err) => return input_and_compile_error(input, err),
    };

    let event_name = args.value();
    let handler_name = &ast.sig.ident;
    let handler_struct_name = syn::Ident::new(
        &format!(
            "__SocketIOMessageHandler_{}_{}",
            event_name.replace("-", "_"),
            handler_name
        ),
        handler_name.span(),
    );

    let output = quote! {
        #ast

        #[allow(non_camel_case_types)]
        pub struct #handler_struct_name;

        impl ::summer_web::handler::SocketIOHandlerRegistrar for #handler_struct_name {
            fn install_socketio_handlers(&self, socket: &::summer_web::socketioxide::extract::SocketRef) {
                socket.on(#event_name, #handler_name);
            }
        }

        ::summer_web::handler::submit! {
            &(#handler_struct_name) as &dyn ::summer_web::handler::SocketIOHandlerRegistrar
        }
    };

    output.into()
}

pub(crate) fn on_fallback(_args: TokenStream, input: TokenStream) -> TokenStream {
    let ast = match syn::parse::<ItemFn>(input.clone()) {
        Ok(ast) => ast,
        Err(err) => return input_and_compile_error(input, err),
    };

    let handler_name = &ast.sig.ident;
    let handler_struct_name = syn::Ident::new(
        &format!("__SocketIOFallbackHandler_{handler_name}"),
        handler_name.span(),
    );

    let output = quote! {
        #ast

        #[allow(non_camel_case_types)]
        pub struct #handler_struct_name;

        impl ::summer_web::handler::SocketIOHandlerRegistrar for #handler_struct_name {
            fn install_socketio_handlers(&self, socket: &::summer_web::socketioxide::extract::SocketRef) {
                socket.on_fallback(#handler_name);
            }
        }

        ::summer_web::handler::submit! {
            &(#handler_struct_name) as &dyn ::summer_web::handler::SocketIOHandlerRegistrar
        }
    };

    output.into()
}
