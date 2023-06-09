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
        .tag("service", "simple-logging-with-auth")
        .build();

    // set the actual logger for the facade
    set_boxed_logger(Box::new(my_loki)).unwrap();
    set_max_level(LevelFilter::Trace);

    // use the regular log macros for actual logging in the app
    trace!("This is a TRACE message");
    debug!("This is a DEBUG message");
    info!("This is a INFO message");
    warn!("This is a WARN message");
    error!("This is a ERROR message");
}
