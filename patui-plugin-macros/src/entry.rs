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
    name: String,
    description: String,
    r#type: String,
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
            let filter = filter.map_or(EnvFilter::default(), EnvFilter::new);

            let writer = match std::env::var("PATUI_LOG_FILE") {
                Ok(path) if !path.is_empty() => {
                    let now = ptplugin::chrono::offset::Local::now();
                    let path = path
                        .replace("${timestamp}", &now.timestamp().to_string())
                        .replace("${datetime}", &now.format("%Y%m%d%H%M%S").to_string());
                    let path = std::path::Path::new(&path);
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    ptplugin::tracing_subscriber::fmt::writer::BoxMakeWriter::new(std::sync::Arc::new(std::fs::File::create(path)?))
                }
                _ => ptplugin::tracing_subscriber::fmt::writer::BoxMakeWriter::new(std::io::stderr),
            };

            let fmt_layer = ptplugin::tracing_subscriber::fmt::layer()
                .with_file(false)
                .with_line_number(false)
                .with_target(true)
                .with_ansi(false)
                .with_writer(writer)
                .without_time();

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
    let name = &config.name;
    let description = &config.description;
    let type_ = &config.r#type;
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
            quote! {
                #name => {
                    let args = request.args;
                    let args = args.into_iter().collect::<std::collections::HashMap<_, _>>();

                    let obj = #path::new(self.results_fully_recieved.clone());
                    obj.run(request.step_name, args, results, produced_results_waker_rx)
                },
            }
        })
        .collect::<Vec<_>>();

    quote! {
        #[derive(Debug)]
        pub(crate) struct #plugin_server_struct_name {
            shutdown_signal: ptplugin::tokio::sync::Mutex<Option<ptplugin::tokio::sync::oneshot::Sender<()>>>,

            results_fully_recieved: std::sync::Arc<std::sync::atomic::AtomicBool>,

            results: std::sync::Arc<ptplugin::tokio::sync::RwLock<ptplugin::PatuiData>>,
            produced_results: std::sync::Arc<ptplugin::tokio::sync::RwLock<Vec<ptplugin::PatuiEvent>>>,
            produced_results_waker:
                ptplugin::tokio::sync::Mutex<Option<(
                    ptplugin::tokio::sync::broadcast::Sender<ptplugin::WakerType>,
                    ptplugin::tokio::sync::broadcast::Receiver<ptplugin::WakerType>
                )>>,
        }

        impl #plugin_server_struct_name {
            pub fn new(shutdown_signal: ptplugin::tokio::sync::oneshot::Sender<()>) -> Self {
                Self {
                    shutdown_signal: ptplugin::tokio::sync::Mutex::new(Some(shutdown_signal)),

                    results_fully_recieved: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),

                    results: std::sync::Arc::new(ptplugin::tokio::sync::RwLock::new(ptplugin::PatuiData::Pending(
                        ptplugin::PatuiDataInner::Map(std::collections::HashMap::new()),
                    ))),
                    produced_results: std::sync::Arc::new(ptplugin::tokio::sync::RwLock::new(Vec::new())),
                    produced_results_waker: ptplugin::tokio::sync::Mutex::new(Some(ptplugin::tokio::sync::broadcast::channel(16))),
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
                        name: #name.to_string(),
                        description: #description.to_string(),
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        r#type: #type_.to_string(),
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

            type RunStream = ptplugin::tokio_stream::wrappers::ReceiverStream<
                std::result::Result<ptplugin::plugin_server::run::Response, ptplugin::tonic::Status>,
            >;

            async fn run(
                &self,
                request: ptplugin::tonic::Request<ptplugin::plugin_server::run::Request>,
            ) -> std::result::Result<ptplugin::tonic::Response<Self::RunStream>, ptplugin::tonic::Status> {
                use ptplugin::FunctionService;

                let request = request.into_inner();

                ptplugin::tracing::info!("Request run {}", request.function);

                let (send_patui_results_tx, send_patui_results_rx) = ptplugin::tokio::sync::mpsc::channel(16);
                let results = self.results.clone();

                let produced_results_waker_rx = self
                    .produced_results_waker
                    .lock()
                    .await
                    .as_ref()
                    .unwrap()
                    .0
                    .subscribe();

                // TODO: Assert that run_task finishes?
                let (produce_results_rx, _run_task) = match request.function.as_str() {
                    #(#function_names)*
                    s => {
                        return Err(ptplugin::tonic::Status::invalid_argument(format!(
                            "Unknown function '{}'",
                            s
                        )));
                    }
                };

                // TODO: Assert this finishes after we get a Done event/in shutdown?
                ptplugin::tokio::spawn(async move {
                    let send_patui_results_tx = send_patui_results_tx;
                    // let mut produced_results = self.produced_results.clone();
                    let mut produce_results_rx = produce_results_rx;

                    while let Some(event_res) = produce_results_rx.recv().await {
                        {
                            // TODO: Add results into self.produced_results/self.results?
                            let result = match event_res {
                                Ok(event) => Ok(ptplugin::plugin_server::run::Response {
                                    data: Some(
                                        (&event)
                                            .try_into()
                                            .expect("Should be able to encode any event"),
                                    ),
                                }),
                                Err(e) => Err(ptplugin::tonic::Status::internal(format!("Error: {}", e))),
                            };

                            ptplugin::tracing::debug!("Sending event details: {:?}", result);

                            if let Err(e) = send_patui_results_tx.send(result).await {
                                ptplugin::tracing::error!("Error sending result: {:?}", e);
                                break;
                            }
                        }
                    }
                });

                Ok(ptplugin::tonic::Response::new(
                    ptplugin::tokio_stream::wrappers::ReceiverStream::new(send_patui_results_rx),
                ))
            }

            type ReceiveResultsStream = std::pin::Pin<
                Box<
                    dyn ptplugin::tokio_stream::Stream<
                            Item = std::result::Result<ptplugin::plugin_server::receive_results::Response, ptplugin::tonic::Status>,
                        > + Send
                        + 'static,
                >,
            >;

            async fn receive_results(
                &self,
                request: ptplugin::tonic::Request<ptplugin::tonic::Streaming<ptplugin::plugin_server::receive_results::Request>>,
            ) -> std::result::Result<ptplugin::tonic::Response<Self::ReceiveResultsStream>, ptplugin::tonic::Status> {
                let produced_results_waker_tx = self
                    .produced_results_waker
                    .lock()
                    .await
                    .as_ref()
                    .unwrap()
                    .0
                    .clone();
                let mut stream = request.into_inner();
                let results = self.results.clone();
                let results_fully_recieved = self.results_fully_recieved.clone();

                let output = ptplugin::async_stream::try_stream! {
                    ptplugin::tracing::trace!("Setup receive results streams");
                    loop {
                        let request = match stream.next().await {
                            Some(Ok(r)) => r,
                            Some(Err(e)) => {
                                ptplugin::tracing::error!("Error receiving results: {:?}", e);
                                break;
                            }
                            None => break,
                        };
                        ptplugin::tracing::trace!("Received results: {:?}", request);

                        let result: ptplugin::PatuiStepResult = request.result.unwrap().try_into().unwrap();
                        ptplugin::tracing::debug!("Received result: {:?}", result);
                        {
                            ptplugin::tracing::trace!("Locking write results");
                            let mut lock = results.write().await;
                            ptplugin::tracing::trace!("Locked write results: {:?}", results);
                            lock.add_step_result_to_stream(&result).unwrap();
                            ptplugin::tracing::trace!("New results: {:?}", lock);

                            // Important we send this before unlocking the results as otherwise we might
                            // get a race condition trying to lock the results stream.
                            produced_results_waker_tx.send(ptplugin::WakerType::Results).unwrap();

                            ptplugin::tracing::trace!("Unlocking write results");
                        }

                        let result = ptplugin::plugin_server::receive_results::Response {
                            diagnostics: vec![],
                        };

                        yield result.clone();
                    }
                    ptplugin::tracing::trace!("Finished receive results streams");
                    results_fully_recieved.store(true, std::sync::atomic::Ordering::SeqCst);
                    produced_results_waker_tx.send(ptplugin::WakerType::Done).unwrap();
                };

                Ok(ptplugin::tonic::Response::new(
                    Box::pin(output) as Self::ReceiveResultsStream
                ))
            }

            async fn shutdown(
                &self,
                request: ptplugin::tonic::Request<ptplugin::plugin_server::shutdown::Request>,
            ) -> std::result::Result<ptplugin::tonic::Response<ptplugin::plugin_server::shutdown::Response>, ptplugin::tonic::Status> {
                ptplugin::tracing::info!("Requesting shutdown: {:?}", request.remote_addr());

                let shutdown_tx = self.shutdown_signal.lock().await.take().unwrap();
                if let Err(e) = shutdown_tx.send(()) {
                    panic!("Error sending shutdown signal: {:?}", e);
                }

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
    if args.is_empty() {
        return Err(syn::Error::new_spanned(args, "expected some arguments"));
    }

    let arg = &args[0];

    let syn::Meta::Path(path) = arg else {
        return Err(syn::Error::new_spanned(arg, "expected a path"));
    };

    let mut name = None;
    let mut description = None;
    let mut r#type = None;
    let mut function_names = vec![];

    for arg in args.iter().skip(1) {
        let name_value = match arg {
            syn::Meta::NameValue(name_value) => name_value,
            _ => {
                return Err(syn::Error::new_spanned(arg, "expected a name-value pair"));
            }
        };
        let ident = name_value
            .path
            .get_ident()
            .ok_or_else(|| syn::Error::new_spanned(name_value, "expected an identifier"))?
            .to_string()
            .to_lowercase();

        match &ident[..] {
            "build_struct" => match &name_value.value {
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
            },
            "name" => {
                name = Some(match &name_value.value {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit_str),
                        ..
                    }) => lit_str.value(),
                    _ => {
                        return Err(syn::Error::new_spanned(
                            name_value,
                            "expected a string literal",
                        ))
                    }
                })
            }
            "description" => {
                description = Some(match &name_value.value {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit_str),
                        ..
                    }) => lit_str.value(),
                    _ => {
                        return Err(syn::Error::new_spanned(
                            name_value,
                            "expected a string literal",
                        ))
                    }
                })
            }
            "r#type" => {
                r#type = Some(match &name_value.value {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit_str),
                        ..
                    }) => lit_str.value(),
                    _ => {
                        return Err(syn::Error::new_spanned(
                            name_value,
                            "expected a string literal",
                        ))
                    }
                })
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    name_value,
                    format!("Unexpected argument name `{}`", ident),
                ))
            }
        }
    }

    let name = name.ok_or_else(|| syn::Error::new_spanned(&args, "missing `name` argument"))?;
    let description = description
        .ok_or_else(|| syn::Error::new_spanned(&args, "missing `description` argument"))?;
    let r#type = r#type.ok_or_else(|| syn::Error::new_spanned(&args, "missing `type` argument"))?;

    Ok(Config {
        plugin_server_struct_name: path.clone(),
        function_names,
        name,
        description,
        r#type,
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
