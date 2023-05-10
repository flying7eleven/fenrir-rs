fn main() {
    use fenrir_rs::FenrirBuilder;
    use fern::Dispatch;
    use log::LevelFilter;
    use log::{debug, error, info, trace, warn};
    use url::Url;

    let my_loki = FenrirBuilder::new(Url::parse("http://localhost:3100").unwrap()).build();

    let _ = Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339(std::time::SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        // just log messages with DEBUG or higher log leven
        .level(log::LevelFilter::Debug)
        // do not log messages from our network library
        .level_for("ureq", LevelFilter::Off)
        // print the log messages to the console ...
        .chain(std::io::stdout())
        // ... and to the corresponding loki endpoint
        .chain(Box::new(my_loki) as Box<dyn log::Log>)
        .apply();

    // use the regular log macros for actual logging in the app
    trace!("This is a TRACE message");
    debug!("This is a DEBUG message");
    info!("This is a INFO message");
    warn!("This is a WARN message");
    error!("This is a ERROR message");
}
