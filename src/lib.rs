use url::Url;

/// The `Fenrir` struct implements the communication interface with a [Loki](https://grafana.com/oss/loki/) instance.
///
/// To create a new instance of the `Fenrir` struct use the `FenrirBuilder` struct.
pub struct Fenrir {
    /// The loki `endpoint` which is used to send log information to
    endpoint: Url,
}

/// The `FenrirBuilder` struct is used to create a new instance of `Fenrir`using the builder pattern.
///
/// This should make it easier and more intuitive (at least I hope) to use the crate without referring
/// to the documentation all the time.
pub struct FenrirBuilder {
    /// The loki `endpoint` which is used to send log information to
    endpoint: Url,
}

impl FenrirBuilder {
    /// Create a new `FenrirBuilder` with all required parameters.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::FenrirBuilder;
    ///
    /// let builder = FenrirBuilder::new(Url::parse("https://loki.example.com").unwrap());
    /// ```
    pub fn new(endpoint: Url) -> FenrirBuilder {
        FenrirBuilder { endpoint }
    }

    /// Create a new `Fenrir` instance with the parameters supplied to this struct before calling `build()`.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::FenrirBuilder;
    ///
    /// let fenrir = FenrirBuilder::new(Url::parse("https://loki.example.com").unwrap()).build();
    /// ```
    pub fn build(self) -> Fenrir {
        Fenrir {
            endpoint: self.endpoint,
        }
    }
}
