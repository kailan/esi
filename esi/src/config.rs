use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Configuration {
    pub namespace: String,
    pub backends: HashMap<String, BackendConfiguration>
}

#[derive(Clone, Debug, Default)]
pub struct BackendConfiguration {
    pub name: Option<String>,
    pub pass: bool,
    pub ttl: Option<u32>,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            namespace: String::from("esi"),
            backends: HashMap::new()
        }
    }
}

impl Configuration {
    /// Sets an alternative ESI namespace, which is used to identify ESI instructions.
    ///
    /// For example, setting this to `test` would cause the processor to only match tags like `<test:include>`.
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Configures a backend to use for requests to the given hostname.
    pub fn with_backend_override(mut self, host: impl Into<String>, backend: impl Into<String>) -> Self {
        let host = host.into();
        if let Some(config) = self.backends.get_mut(&host) {
            config.name = Some(backend.into());
        } else {
            self.backends.insert(host, BackendConfiguration {
                name: Some(backend.into()),
                ..Default::default()
            });
        }
        self
    }

    /// Configures request settings for any requests to the given hostname.
    pub fn with_backend(mut self, host: impl Into<String>, backend: BackendConfiguration) -> Self {
        let host = host.into();
        if let Some(config) = self.backends.get_mut(&host) {
            *config = backend;
        } else {
            self.backends.insert(host, backend);
        }
        self
    }
}
