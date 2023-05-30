fn main() {
    use fenrir_rs::{AuthenticationMethod, Fenrir, NetworkingBackend, SerializationFormat};
    use log::{debug, error, info, set_boxed_logger, set_max_level, trace, warn, LevelFilter};
    use url::Url;

    let my_loki = Fenrir::builder()
        .endpoint(Url::parse("http://localhost:8080").unwrap())
        .network(NetworkingBackend::Ureq)
        .with_authentication(
            AuthenticationMethod::Basic,
            "example".to_string(),
            "password".to_string(),
        )
        .format(SerializationFormat::Json)
        .include_level()
        .include_framework()
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
