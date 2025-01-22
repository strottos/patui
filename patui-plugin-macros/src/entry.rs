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
            let var_name = EnvFilter::default();
            let filter = filter.map_or(var_name, EnvFilter::new);

            let fmt_layer = ptplugin::tracing_subscriber::fmt::layer()
                .with_file(false)
                .with_line_number(false)
                .with_target(true)
                .with_ansi(false)
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
            quote! { #name => Box::new(#path), }
        })
        .collect::<Vec<_>>();

    quote! {
        #[derive(Debug)]
        pub(crate) struct #plugin_server_struct_name {
            shutdown_signal: ptplugin::tokio::sync::Mutex<Option<ptplugin::tokio::sync::oneshot::Sender<()>>>,

            results: std::sync::Arc<ptplugin::tokio::sync::Mutex<ptplugin::PatuiData>>,

            produced_results_channel: (
                std::sync::Mutex<Option<ptplugin::tokio::sync::mpsc::Sender<(String, ptplugin::PatuiEvent)>>>,
                std::sync::Mutex<Option<ptplugin::tokio::sync::mpsc::Receiver<(String, ptplugin::PatuiEvent)>>>,
            ),

            waker_channel: (
                std::sync::Mutex<Option<ptplugin::tokio::sync::mpsc::Sender<()>>>,
                std::sync::Mutex<Option<ptplugin::tokio::sync::mpsc::Receiver<()>>>,
            ),

            run_done_channel: (
                std::sync::Mutex<Option<ptplugin::tokio::sync::oneshot::Sender<()>>>,
                std::sync::Mutex<Option<ptplugin::tokio::sync::oneshot::Receiver<()>>>,
            ),
            produce_results_done_channel: (
                std::sync::Mutex<Option<ptplugin::tokio::sync::oneshot::Sender<()>>>,
                std::sync::Mutex<Option<ptplugin::tokio::sync::oneshot::Receiver<()>>>,
            ),
            receive_results_done_channel: (
                std::sync::Mutex<Option<ptplugin::tokio::sync::oneshot::Sender<()>>>,
                std::sync::Mutex<Option<ptplugin::tokio::sync::oneshot::Receiver<()>>>,
            ),
            receive_results_cancel_tx:
                std::sync::Mutex<Option<ptplugin::tokio::sync::broadcast::Sender<()>>>,

            expected_acks: std::sync::Arc<ptplugin::tokio::sync::Mutex<std::collections::HashMap<i64, ptplugin::tokio::sync::oneshot::Sender<()>>>>,
        }

        impl #plugin_server_struct_name {
            pub fn new(shutdown_signal: ptplugin::tokio::sync::oneshot::Sender<()>) -> Self {
                let (waker_tx, waker_rx) = ptplugin::tokio::sync::mpsc::channel(1);

                let (produced_results_tx, produced_results_rx) = ptplugin::tokio::sync::mpsc::channel(1);

                let (run_done_oneshot_tx, run_done_oneshot_rx) = ptplugin::tokio::sync::oneshot::channel();
                let (produce_results_done_tx, produce_results_done_rx) = ptplugin::tokio::sync::oneshot::channel();
                let (receive_results_done_tx, receive_results_done_rx) = ptplugin::tokio::sync::oneshot::channel();
                let (receive_results_cancel_tx, _) = ptplugin::tokio::sync::broadcast::channel(1);

                Self {
                    shutdown_signal: ptplugin::tokio::sync::Mutex::new(Some(shutdown_signal)),

                    results: std::sync::Arc::new(ptplugin::tokio::sync::Mutex::new(ptplugin::PatuiData::Pending(ptplugin::PatuiDataInner::Map(std::collections::HashMap::new())))),

                    produced_results_channel: (
                        std::sync::Mutex::new(Some(produced_results_tx)),
                        std::sync::Mutex::new(Some(produced_results_rx)),
                    ),

                    waker_channel: (
                        std::sync::Mutex::new(Some(waker_tx)),
                        std::sync::Mutex::new(Some(waker_rx)),
                    ),

                    run_done_channel: (
                        std::sync::Mutex::new(Some(run_done_oneshot_tx)),
                        std::sync::Mutex::new(Some(run_done_oneshot_rx)),
                    ),
                    produce_results_done_channel: (
                        std::sync::Mutex::new(Some(produce_results_done_tx)),
                        std::sync::Mutex::new(Some(produce_results_done_rx)),
                    ),
                    receive_results_done_channel: (
                        std::sync::Mutex::new(Some(receive_results_done_tx)),
                        std::sync::Mutex::new(Some(receive_results_done_rx)),
                    ),
                    receive_results_cancel_tx:
                        std::sync::Mutex::new(Some(receive_results_cancel_tx)),

                    expected_acks: std::sync::Arc::new(ptplugin::tokio::sync::Mutex::new(std::collections::HashMap::new())),
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

            async fn run(
                &self,
                request: ptplugin::tonic::Request<ptplugin::plugin_server::run::Request>,
            ) -> std::result::Result<ptplugin::tonic::Response<ptplugin::plugin_server::run::Response>, ptplugin::tonic::Status> {
                let request = request.into_inner();

                ptplugin::tracing::info!("Request run {}", request.function);

                let function_service: Box<dyn ptplugin::FunctionService> = match request.function.as_str() {
                    #(#function_names)*
                    _ => {
                        return Err(ptplugin::tonic::Status::unimplemented(
                            &format!("Function {} not implemented for running", request.function),
                        ))
                    }
                };

                ptplugin::tracing::info!("Running function: {}", request.function);

                let produced_results_tx = self.produced_results_channel.0.lock().unwrap().take().unwrap();
                let waker_rx = self.waker_channel.1.lock().unwrap().take().unwrap();

                match function_service.run(request.args.clone(), produced_results_tx, self.results.clone(), waker_rx) {
                    Ok(task) => {
                        let run_done_tx = self.run_done_channel.0.lock().unwrap().take().unwrap();
                        let receive_results_cancel_tx = self.receive_results_cancel_tx.lock().unwrap().clone().unwrap();
                        ptplugin::tokio::spawn(async move {
                            ptplugin::tracing::trace!("Awaiting run task completion");
                            if let Err(e) = task.await {
                                panic!("Error waiting for run function: {:?}", e);
                            }
                            ptplugin::tracing::trace!("Run task complete");
                            if let Err(e) = run_done_tx.send(()) {
                                tracing::warn!("Error notifying run task done: {:?}", e);
                            }
                            let _ = receive_results_cancel_tx.send(());
                            ptplugin::tracing::trace!("Run task done notified");
                        });
                    }
                    Err(e) => {
                        ptplugin::tracing::error!("Error running function: {:?}", e);
                        return Err(ptplugin::tonic::Status::internal(format!("Error running function: {}", e)));
                    }
                };

                Ok(ptplugin::tonic::Response::new(ptplugin::plugin_server::run::Response {
                    diagnostics: vec![],
                }))
            }

            type ProduceResultsStream = ptplugin::tokio_stream::wrappers::ReceiverStream<std::result::Result<ptplugin::plugin_server::produce_results::Request, ptplugin::tonic::Status>>;

            async fn produce_results(
                &self,
                _request: ptplugin::tonic::Request<ptplugin::plugin_server::produce_results::Init>,
            ) -> std::result::Result<ptplugin::tonic::Response<Self::ProduceResultsStream>, ptplugin::tonic::Status> {
                let produced_results_rx = self.produced_results_channel.1.lock().unwrap().take().unwrap();

                let (tx, rx) = ptplugin::tokio::sync::mpsc::channel(1);
                let expected_acks = self.expected_acks.clone();
                let done_tx = self.produce_results_done_channel.0.lock().unwrap().take().unwrap();
                let counter = std::sync::atomic::AtomicI64::new(1);

                ptplugin::tokio::spawn(async move {
                    let mut produced_results_rx = produced_results_rx;
                    let mut tasks = vec![];
                    while let Some((name, event)) = produced_results_rx.recv().await {
                        let id = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        ptplugin::tracing::info!("Event to send - {}: {:?}", name, event);
                        let result = event.try_into();
                        let result = match result {
                            Ok(r) => {
                                Ok(ptplugin::plugin_server::produce_results::Request {
                                    id,
                                    name,
                                    data: Some(r),
                                    diagnostics: vec![],
                                })
                            },
                            Err(e) => {
                                Err(ptplugin::tonic::Status::internal(format!("Error converting result: {}", e)))
                            },
                        };
                        ptplugin::tracing::debug!("Sending event details: {:?}", result);
                        let (ack_tx, ack_rx) = ptplugin::tokio::sync::oneshot::channel();

                        expected_acks.lock().await.insert(id, ack_tx);

                        ptplugin::tracing::trace!("HERE1");
                        if let Err(e) = tx.send(result).await {
                            panic!("Error sending result: {:?}", e);
                        };
                        ptplugin::tracing::trace!("HERE2");

                        // Ensure we get an ack for the result, otherwise these tasks will fail or
                        // timeout.
                        tasks.push(ptplugin::tokio::spawn(async move {
                        ptplugin::tracing::trace!("HERE3");
                            if let Err(e) = ack_rx.await {
                                panic!("Error waiting for ack: {:?}", e);

                            }
                        ptplugin::tracing::trace!("HERE4");
                        }));
                    }

                    for task in tasks.drain(..) {
                        ptplugin::tracing::trace!("HERE5");
                        ptplugin::tracing::trace!("Waiting for ack");
                        if let Err(e) = task.await {
                            panic!("Error waiting for ack task: {:?}", e);
                        }
                        ptplugin::tracing::trace!("HERE6");
                        ptplugin::tracing::trace!("Ack received");
                    }

                        ptplugin::tracing::trace!("HERE7");
                    if let Err(e) = done_tx.send(()) {
                        panic!("Error sending produce results done: {:?}", e);
                    }
                        ptplugin::tracing::trace!("HERE8");
                });

                Ok(ptplugin::tonic::Response::new(ptplugin::tokio_stream::wrappers::ReceiverStream::new(rx)))
            }

            async fn ack_result(
                &self,
                request: ptplugin::tonic::Request<ptplugin::plugin_server::ack_result::Request>,
            ) -> std::result::Result<ptplugin::tonic::Response<ptplugin::plugin_server::ack_result::Response>, ptplugin::tonic::Status> {
                let request = request.into_inner();


                ptplugin::tracing::trace!("Ack result: {}", request.id);
                if let Err(e) = self.expected_acks.lock().await.remove(&request.id).unwrap().send(()) {
                    panic!("Error sending ack {}: {:?}", request.id, e);
                }
                ptplugin::tracing::trace!("Acked: {}", request.id);

                Ok(ptplugin::tonic::Response::new(ptplugin::plugin_server::ack_result::Response {}))
            }

            type ReceiveResultsStream =
                std::pin::Pin<Box<dyn ptplugin::tokio_stream::Stream<Item = std::result::Result<ptplugin::plugin_server::receive_results::Response, ptplugin::tonic::Status>> + Send + 'static>>;

            async fn receive_results(
                &self,
                request: ptplugin::tonic::Request<ptplugin::tonic::Streaming<ptplugin::plugin_server::receive_results::Request>>,
            ) -> std::result::Result<ptplugin::tonic::Response<Self::ReceiveResultsStream>, ptplugin::tonic::Status> {
                ptplugin::tracing::info!("Receive results: {:?}", request.remote_addr());
                let mut stream = request.into_inner();
                let results = self.results.clone();
                let waker_tx = self.waker_channel.0.lock().unwrap().take().unwrap();
                let done_tx = self.receive_results_done_channel.0.lock().unwrap().take().unwrap();
                let receive_results_cancel_tx = self.receive_results_cancel_tx.lock().unwrap().clone().unwrap();

                let output = ptplugin::async_stream::try_stream! {
                    loop {
                        let mut receive_results_cancel_rx = receive_results_cancel_tx.subscribe();
                        tracing::trace!("HELLO1");
                        ptplugin::tokio::select! {
                            message = stream.next() => {
                                tracing::trace!("HELLO2");
                                let message = match message {
                                    Some(Ok(message)) => message,
                                    _ => break,
                                };

                                ptplugin::tracing::info!("Results received: {:?}", message);

                                let data: ptplugin::PatuiData = message.results.unwrap().try_into().unwrap();

                                ptplugin::tracing::debug!("Results: {:?}", data);

                                let mut lock = results.lock().await;
                                *lock = data;
                                if let Err(e) = waker_tx.send(()).await {
                                    panic!("Error sending wakeup call: {:?}", e);
                                }

                                let result = ptplugin::plugin_server::receive_results::Response {
                                    diagnostics: vec![],
                                };

                                yield result.clone();
                            }
                            done = receive_results_cancel_rx.recv() => {
                                tracing::trace!("HELLO3");
                                break;
                            }
                        }
                    }

                    ptplugin::tracing::info!("Publish stream ended");

                    // TODO: possibly Patui should make this decision
                    // let mut lock = results.lock().await;
                    // *lock = match lock.clone().to_known() {
                    //     Ok(res) => res,
                    //     Err(e) => {
                    //         panic!("Error converting results to known: {:?}", e);
                    //     }
                    // };
                    // if let Err(e) = waker_tx.send(()).await {
                    //     panic!("Error sending final wakeup call: {:?}", e);
                    // }

                    if let Err(e) = done_tx.send(()) {
                        panic!("Error sending receive results done: {:?}", e);
                    }
                };

                Ok(ptplugin::tonic::Response::new(Box::pin(output) as Self::ReceiveResultsStream))
            }

            async fn wait(
                &self,
                request: ptplugin::tonic::Request<ptplugin::plugin_server::wait::Request>,
            ) -> std::result::Result<ptplugin::tonic::Response<ptplugin::plugin_server::wait::Response>, ptplugin::tonic::Status> {
                ptplugin::tracing::info!("Request wait: {:?}", request.remote_addr());

                let receive_results_done_rx = self.receive_results_done_channel.1.lock().unwrap().take().unwrap();

                if let Err(e) = receive_results_done_rx.await {
                    panic!("Error waiting for receive results done: {:?}", e);
                }
                ptplugin::tracing::trace!("Receive results notify received");

                let mut run_done_rx = self.run_done_channel.1.lock().unwrap().take().unwrap();
                                tracing::trace!("HELLO4");
                if let Err(e) = run_done_rx.await {
                                tracing::trace!("HELLO5");
                    panic!("Error waiting for run done: {:?}", e);
                }
                ptplugin::tracing::trace!("Run done notify received");

                let produce_results_done_rx = self.produce_results_done_channel.1.lock().unwrap().take().unwrap();
                if let Err(e) = produce_results_done_rx.await {
                    panic!("Error waiting for produce results done: {:?}", e);
                }
                ptplugin::tracing::trace!("Produce results done notify received");
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
