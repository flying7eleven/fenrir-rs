fn main() {
    use fenrir_rs::{Fenrir, NetworkingBackend, SerializationFormat};
    use log::{debug, error, info, set_boxed_logger, set_max_level, trace, warn, LevelFilter};
    use url::Url;

    let my_loki = Fenrir::builder()
        .endpoint(Url::parse("http://localhost:3100").unwrap())
        .network(NetworkingBackend::Ureq)
        .format(SerializationFormat::Json)
        .include_level()
        .tag("service", "structured-logging")
        .build();

    // set the actual logger for the facade
    set_boxed_logger(Box::new(my_loki)).unwrap();
    set_max_level(LevelFilter::Trace);

    // use the regular log macros for actual logging in the app
    trace!(app = "structured-logging"; "This is a TRACE message");
    debug!(app = "structured-logging"; "This is a DEBUG message");
    info!(app = "structured-logging"; "This is a INFO message");
    warn!(app = "structured-logging", critical = true; "This is a WARN message");
    error!(app = "structured-logging", fatal = false; "This is a ERROR message");
}
