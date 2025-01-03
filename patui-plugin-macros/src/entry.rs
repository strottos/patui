use convert_case::Casing;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    braced,
    parse::{Parse, ParseStream, Parser},
    Path, Signature,
};

// Idea taken from https://github.com/tokio-rs/tokio/blob/2052938a9f743b308f193665f8ccedd827c139b1/tokio-macros/src/entry.rs#L6tokio-macros/src/entry.rs
type AttributeArgs = syn::punctuated::Punctuated<syn::Meta, syn::Token![,]>;

#[derive(Debug)]
struct Config {
    plugin_server_struct_name: Path,
    function_names: Vec<Path>,
}

struct ItemFn {
    sig: Signature,
    brace_token: syn::token::Brace,
    stmts: Vec<TokenStream>,
}

impl Parse for ItemFn {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let sig: Signature = input.parse()?;
        let content;
        let brace_token = braced!(content in input);
        let mut stmts = Vec::new();

        while !content.is_empty() {
            let stmt = content.parse()?;
            stmts.push(stmt);
        }

        Ok(Self {
            sig,
            brace_token,
            stmts,
        })
    }
}

/// Generate the logging initialisation function.
fn init_logging_function() -> TokenStream {
    quote! {
        fn initialise_logging() -> ptplugin::Result<()> {
            use ptplugin::tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

            let filter = match std::env::var("PATUI_LOG") {
                Ok(log) => Some(log),
                Err(_) => return Ok(()),
            };
            let var_name = EnvFilter::default();
            let filter = filter.map_or(var_name, EnvFilter::new);

            let fmt_layer = ptplugin::tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_target(true)
                .with_ansi(true);

            Registry::default().with(filter).with(fmt_layer).init();

            Ok(())
        }
    }
}

fn init_clap_data_structures() -> TokenStream {
    quote! {
        fn clap_matches() -> ptplugin::clap::ArgMatches {
            use ptplugin::clap::{arg, command, value_parser, ArgAction, Command};

            ptplugin::clap::command!()
                .arg(
                    arg!(
                        -p --port <FILE> "Sets a custom config file"
                    )
                    .required(true)
                    .value_parser(value_parser!(u16)),
                )
                .after_help("This is a Patui plugin and should not be called directly, please use Patui to orchestrate.\nFor further information please visit https://patui.dev/docs")
                .get_matches()
        }
    }
}

fn init_server_structures(config: &Config) -> TokenStream {
    if config.function_names.is_empty() {
        return quote! {};
    }

    let plugin_server_struct_name = &config.plugin_server_struct_name;
    let function_names = config
        .function_names
        .iter()
        .map(|path| {
            let name = path
                .segments
                .last()
                .unwrap()
                .ident
                .to_string()
                .to_case(convert_case::Case::Snake);
            quote! { #name => Box::new(#path), }
        })
        .collect::<Vec<_>>();

    quote! {
        #[derive(Debug)]
        pub(crate) struct #plugin_server_struct_name {
            tasks: std::sync::Arc<std::sync::Mutex<Vec<ptplugin::tokio::task::JoinHandle<()>>>>,
            shutdown_signal: ptplugin::tokio::sync::Mutex<Option<ptplugin::tokio::sync::oneshot::Sender<()>>>,

            results: std::sync::Arc<ptplugin::tokio::sync::Mutex<ptplugin::PatuiData>>,

            waker_tx: std::sync::Mutex<Option<ptplugin::tokio::sync::mpsc::Sender<()>>>,
            waker_rx: std::sync::Mutex<Option<ptplugin::tokio::sync::mpsc::Receiver<()>>>,
        }

        impl #plugin_server_struct_name {
            pub fn new(shutdown_signal: ptplugin::tokio::sync::oneshot::Sender<()>) -> Self {
                let (waker_tx, waker_rx) = ptplugin::tokio::sync::mpsc::channel(1);

                Self {
                    tasks: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
                    shutdown_signal: ptplugin::tokio::sync::Mutex::new(Some(shutdown_signal)),

                    results: std::sync::Arc::new(ptplugin::tokio::sync::Mutex::new(ptplugin::PatuiData::Pending(ptplugin::PatuiDataInner::Map(std::collections::HashMap::new())))),

                    waker_tx: std::sync::Mutex::new(Some(waker_tx)),
                    waker_rx: std::sync::Mutex::new(Some(waker_rx)),
                }
            }
        }

        use ptplugin::tokio_stream::StreamExt;

        #[ptplugin::tonic::async_trait]
        impl ptplugin::plugin_server::PluginService for #plugin_server_struct_name {
            async fn get_info(
                &self,
                request: ptplugin::tonic::Request<ptplugin::plugin_server::get_info::Request>,
            ) -> std::result::Result<ptplugin::tonic::Response<ptplugin::plugin_server::get_info::Response>, ptplugin::tonic::Status> {
                ptplugin::tracing::info!("Request get_info: {:?}", request);

                let reply = ptplugin::plugin_server::get_info::Response {
                    step_runner: Some(ptplugin::plugin_server::StepRunner {
                        name: "patui_std".to_string(),
                        description: "Patui Standard Plugin, standard Patui utilities like assertions and file manipulations".to_string(),
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        r#type: "std".to_string(),
                        subscriptions: vec![],
                    }),
                };
                Ok(ptplugin::tonic::Response::new(reply))
            }

            async fn init(
                &self,
                request: ptplugin::tonic::Request<ptplugin::plugin_server::init::Request>,
            ) -> std::result::Result<ptplugin::tonic::Response<ptplugin::plugin_server::init::Response>, ptplugin::tonic::Status> {
                ptplugin::tracing::info!("Request init: {:?}", request.remote_addr());

                Ok(ptplugin::tonic::Response::new(ptplugin::plugin_server::init::Response {
                    diagnostics: vec![],
                }))
            }

            type RunStream = ptplugin::tokio_stream::wrappers::ReceiverStream<std::result::Result<ptplugin::plugin_server::run::Response, ptplugin::tonic::Status>>;

            async fn run(
                &self,
                request: ptplugin::tonic::Request<ptplugin::plugin_server::run::Request>,
            ) -> std::result::Result<ptplugin::tonic::Response<Self::RunStream>, ptplugin::tonic::Status> {
                let request = request.into_inner();

                ptplugin::tracing::info!("Request run {}", request.function);

                let function_service: Box<dyn ptplugin::FunctionService> = match request.function.as_str() {
                    #(#function_names)*
                    _ => {
                        return Err(ptplugin::tonic::Status::unimplemented(
                            "Subscription for function and name not implemented",
                        ))
                    }
                };

                ptplugin::tracing::info!("Running function: {}", request.function);

                let (resp_tx, resp_rx) = ptplugin::tokio::sync::mpsc::channel(32); // TODO: Configurable

                let waker_rx = self.waker_rx.lock().unwrap().take().unwrap();

                match function_service.run(request.args, resp_tx, self.results.clone(), waker_rx) {
                    Ok(task) => {
                        let mut lock = self.tasks.lock().unwrap();
                        lock.push(task);
                    }
                    Err(e) => {
                        ptplugin::tracing::error!("Error running function: {:?}", e);
                        return Err(ptplugin::tonic::Status::internal(format!("Error running function: {}", e)));
                    }
                };

                Ok(ptplugin::tonic::Response::new(ptplugin::tokio_stream::wrappers::ReceiverStream::new(resp_rx)))
            }

            type PublishStream =
                std::pin::Pin<Box<dyn ptplugin::tokio_stream::Stream<Item = std::result::Result<ptplugin::plugin_server::publish::Response, ptplugin::tonic::Status>> + Send + 'static>>;

            async fn publish(
                &self,
                request: ptplugin::tonic::Request<ptplugin::tonic::Streaming<ptplugin::plugin_server::publish::Request>>,
            ) -> std::result::Result<ptplugin::tonic::Response<Self::PublishStream>, ptplugin::tonic::Status> {
                ptplugin::tracing::info!("Publish: {:?}", request.remote_addr());
                let mut stream = request.into_inner();
                let results = self.results.clone();
                let waker_tx = self.waker_tx.lock().unwrap().as_ref().unwrap().clone();

                let output = ptplugin::async_stream::try_stream! {
                    while let Some(Ok(message)) = stream.next().await {
                        ptplugin::tracing::info!("Message published: {:?}", message);

                        let data: ptplugin::PatuiData = message.data.unwrap().try_into().unwrap();

                        ptplugin::tracing::debug!("Data: {:?}", data);

                        let mut lock = results.lock().await;
                        *lock = data;
                        waker_tx.send(()).await.unwrap();

                        let result = ptplugin::plugin_server::publish::Response {
                            diagnostics: vec![],
                        };

                        yield result.clone();
                    }

                    ptplugin::tracing::info!("Publish stream ended");

                    let mut lock = results.lock().await;
                    *lock = lock.clone().to_known().unwrap();
                    waker_tx.send(()).await.unwrap();
                };

                Ok(ptplugin::tonic::Response::new(Box::pin(output) as Self::PublishStream))
            }

            async fn wait(
                &self,
                request: ptplugin::tonic::Request<ptplugin::plugin_server::wait::Request>,
            ) -> std::result::Result<ptplugin::tonic::Response<ptplugin::plugin_server::wait::Response>, ptplugin::tonic::Status> {
                ptplugin::tracing::info!("Request wait: {:?}", request.remote_addr());

                let mut tasks = vec![];

                {
                    let mut lock = self.tasks.lock().unwrap();
                    for task in lock.drain(..) {
                        tasks.push(task);
                    }
                }

                for task in tasks {
                    ptplugin::tracing::info!("Waiting for task to complete");
                    task.await.unwrap();
                }

                ptplugin::tracing::info!("Done waiting");

                Ok(ptplugin::tonic::Response::new(ptplugin::plugin_server::wait::Response {
                    diagnostics: vec![],
                }))
            }

            async fn shutdown(
                &self,
                request: ptplugin::tonic::Request<ptplugin::plugin_server::shutdown::Request>,
            ) -> std::result::Result<ptplugin::tonic::Response<ptplugin::plugin_server::shutdown::Response>, ptplugin::tonic::Status> {
                ptplugin::tracing::info!("Requesting shutdown: {:?}", request.remote_addr());

                let shutdown_tx = self.shutdown_signal.lock().await.take().unwrap();
                shutdown_tx.send(()).unwrap();

                Ok(ptplugin::tonic::Response::new(ptplugin::plugin_server::shutdown::Response {}))
            }
        }
    }
}

fn main_body_run_server(config: &Config) -> TokenStream {
    let plugin_server_struct_name = &config.plugin_server_struct_name;

    quote! {
        initialise_logging()?;
        let matches = clap_matches();
        let port = matches.get_one::<u16>("port").unwrap();
        ptplugin::tracing::info!("Starting Patui Test Plugin on port {}", port);
        let addr = format!("[::1]:{}", port);
        let addr = addr.parse().unwrap();

        let body = async {
            let (shutdown_tx, shutdown_rx) = ptplugin::tokio::sync::oneshot::channel();

            let plugin_server = <#plugin_server_struct_name>::new(shutdown_tx);
            ptplugin::tonic::transport::Server::builder()
                .add_service(ptplugin::plugin_server::PluginServiceServer::new(plugin_server))
                .serve_with_shutdown(addr, async {
                    shutdown_rx.await.ok();
                    ptplugin::tracing::info!("Shutting down");
                })
                .await
        };

        ptplugin::tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime")
            .block_on(body)
            .map_err(|e| ptplugin::eyre!("Error running server: {}", e))
    }
}

fn check_main_sig(sig: &Signature) -> syn::Result<()> {
    if sig.ident != "main" {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "expected main() function",
        ));
    }

    if !sig.inputs.is_empty() {
        return Err(syn::Error::new_spanned(
            &sig.inputs,
            "expected main() function with no arguments",
        ));
    }

    match sig.output {
        syn::ReturnType::Default => {
            return Err(syn::Error::new_spanned(
                sig,
                "expected main() function to return a ptplugin::Result<()>",
            ));
        }
        syn::ReturnType::Type(_, ref ty) => {
            if let syn::Type::Path(ref ty) = **ty {
                if ty.path.segments.len() != 2
                    || &ty.path.segments[0].ident.to_string() != "ptplugin"
                    || &ty.path.segments[1].ident.to_string() != "Result"
                {
                    return Err(syn::Error::new_spanned(
                        ty,
                        "expected main() function to return a ptplugin::Result<()>",
                    ));
                }
            } else {
                return Err(syn::Error::new_spanned(
                    ty,
                    "expected main() function to return a ptplugin::Result<()>",
                ));
            }
        }
    }

    Ok(())
}

fn token_stream_with_error(mut tokens: TokenStream, error: syn::Error) -> TokenStream {
    tokens.extend(error.into_compile_error());
    tokens
}

fn build_config(args: AttributeArgs) -> Result<Config, syn::Error> {
    if args.is_empty() || args.len() > 2 {
        return Err(syn::Error::new_spanned(
            args,
            "expected one or two argument",
        ));
    }

    let arg = &args[0];

    let syn::Meta::Path(path) = arg else {
        return Err(syn::Error::new_spanned(arg, "expected a path"));
    };

    let mut function_names = vec![];

    if let Some(syn::Meta::NameValue(name_value)) = args.get(1) {
        let ident = name_value
            .path
            .get_ident()
            .ok_or_else(|| syn::Error::new_spanned(name_value, "expected an identifier"))?
            .to_string()
            .to_lowercase();

        if ident != "build_struct" {
            return Err(syn::Error::new_spanned(
                name_value,
                "expected `build_struct` argument",
            ));
        }

        match &name_value.value {
            syn::Expr::Paren(syn::ExprParen { expr, .. }) => {
                if let syn::Expr::Path(syn::ExprPath { path, .. }) = &**expr {
                    function_names.push(path.clone());
                } else {
                    return Err(syn::Error::new_spanned(
                        name_value,
                        "expected a path or an array of paths",
                    ));
                }
            }
            syn::Expr::Tuple(syn::ExprTuple { elems, .. }) => {
                for elem in elems {
                    if let syn::Expr::Path(syn::ExprPath { path, .. }) = elem {
                        function_names.push(path.clone());
                    } else {
                        return Err(syn::Error::new_spanned(
                            name_value,
                            "expected a path or an array of paths",
                        ));
                    }
                }
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    name_value,
                    "expected an array of paths",
                ));
            }
        }
    }

    Ok(Config {
        plugin_server_struct_name: path.clone(),
        function_names,
    })
}

pub fn main(args: TokenStream, item: TokenStream) -> TokenStream {
    let input: ItemFn = match syn::parse2(item.clone()) {
        Ok(it) => it,
        Err(e) => return token_stream_with_error(item, e),
    };

    if let Err(e) = check_main_sig(&input.sig) {
        return token_stream_with_error(item, e);
    }

    let config = match AttributeArgs::parse_terminated
        .parse2(args)
        .and_then(build_config)
    {
        Ok(it) => it,
        Err(e) => return token_stream_with_error(item, e),
    };

    let mut tokens = proc_macro2::TokenStream::new();

    tokens.extend(init_logging_function());
    tokens.extend(init_clap_data_structures());
    tokens.extend(init_server_structures(&config));

    input.sig.to_tokens(&mut tokens);
    input.brace_token.surround(&mut tokens, |tokens| {
        tokens.extend(main_body_run_server(&config));
    });

    tokens
}
