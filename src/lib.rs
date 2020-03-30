//! Python bindings for Brave's adblocking library, which is written in Rust.
#![deny(
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_qualifications
)]

use adblock::blocker::BlockerError as RustBlockerError;
use adblock::blocker::BlockerResult as RustBlockerResult;
use adblock::engine::Engine as RustEngine;
use failure::Fail;
use pyo3::class::PyObjectProtocol;
use pyo3::exceptions::ValueError as PyValueError;
use pyo3::prelude::*;
use pyo3::PyErr;

use std::fs;
use std::io::{Read, Write};

/// Brave's adblocking library in Python!
#[pymodule]
fn adblock(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_class::<Engine>()?;
    Ok(())
}

#[pyclass]
pub struct BlockerResult {
    #[pyo3(get)]
    pub matched: bool,
    #[pyo3(get)]
    pub explicit_cancel: bool,
    #[pyo3(get)]
    pub important: bool,
    #[pyo3(get)]
    pub redirect: Option<String>,
    #[pyo3(get)]
    pub exception: Option<String>,
    #[pyo3(get)]
    pub filter: Option<String>,
    #[pyo3(get)]
    pub error: Option<String>,
}

impl Into<BlockerResult> for RustBlockerResult {
    fn into(self) -> BlockerResult {
        BlockerResult {
            matched: self.matched,
            explicit_cancel: self.explicit_cancel,
            important: self.important,
            redirect: self.redirect,
            exception: self.exception,
            filter: self.filter,
            error: self.error,
        }
    }
}

#[pyproto]
impl PyObjectProtocol for BlockerResult {
    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "BlockerResult({}, {}, {}, {:?}, {:?}, {:?}, {:?})",
            self.matched,
            self.explicit_cancel,
            self.important,
            self.redirect,
            self.exception,
            self.filter,
            self.error
        ))
    }
}

#[derive(Fail, Debug, PartialEq, Copy, Clone)]
pub enum BlockerError {
    #[fail(display = "Serialization error")]
    SerializationError,
    #[fail(display = "Deserialization error")]
    DeserializationError,
    #[fail(display = "Optimized filter exists")]
    OptimizedFilterExistence,
    #[fail(display = "Bad filter add unsupported")]
    BadFilterAddUnsupported,
    #[fail(display = "Filter exists")]
    FilterExists,
}

impl Into<PyErr> for BlockerError {
    fn into(self) -> PyErr {
        PyErr::new::<PyValueError, _>(format!("{:?}", self))
    }
}

impl Into<BlockerError> for RustBlockerError {
    fn into(self) -> BlockerError {
        match self {
            Self::SerializationError => BlockerError::SerializationError,
            Self::DeserializationError => BlockerError::DeserializationError,
            Self::OptimizedFilterExistence => BlockerError::OptimizedFilterExistence,
            Self::BadFilterAddUnsupported => BlockerError::BadFilterAddUnsupported,
            Self::FilterExists => BlockerError::FilterExists,
        }
    }
}

#[pyclass]
pub struct Engine {
    engine: RustEngine,
}

#[pymethods]
impl Engine {
    #[new]
    pub fn from_rules(network_filters: Vec<String>) -> Self {
        let engine = RustEngine::from_rules(&network_filters);
        Self { engine }
    }

    /// ## Request types
    /// Examples of valid `request_type` parameters include:
    /// * `beacon`
    /// * `csp_report`
    /// * `document`
    /// * `font`
    /// * `media`
    /// * `object`
    /// * `script`
    /// * `stylesheet`
    /// * and et cetera...
    pub fn check_network_urls(
        &self,
        url: &str,
        source_url: &str,
        request_type: &str,
    ) -> BlockerResult {
        let blocker_result = self
            .engine
            .check_network_urls(url, source_url, request_type);
        blocker_result.into()
    }

    pub fn check_network_urls_with_hostnames(
        &self,
        url: &str,
        hostname: &str,
        source_hostname: &str,
        request_type: &str,
        third_party_request: Option<bool>,
    ) -> BlockerResult {
        let blocker_result = self.engine.check_network_urls_with_hostnames(
            url,
            hostname,
            source_hostname,
            request_type,
            third_party_request,
        );
        blocker_result.into()
    }

    pub fn check_network_urls_with_hostnames_subset(
        &self,
        url: &str,
        hostname: &str,
        source_hostname: &str,
        request_type: &str,
        third_party_request: Option<bool>,
        previously_matched_rule: bool,
        force_check_exceptions: bool,
    ) -> BlockerResult {
        let blocker_result = self.engine.check_network_urls_with_hostnames_subset(
            url,
            hostname,
            source_hostname,
            request_type,
            third_party_request,
            previously_matched_rule,
            force_check_exceptions,
        );
        blocker_result.into()
    }

    pub fn serialize(&mut self) -> PyResult<Vec<u8>> {
        let result = self.engine.serialize();
        match result {
            Ok(x) => Ok(x),
            Err(error) => {
                let my_blocker_error: BlockerError = error.into();
                Err(my_blocker_error.into())
            }
        }
    }

    pub fn serialize_to_file(&mut self, file: &str) -> PyResult<()> {
        let mut fd = fs::File::open(file)?;
        let data = self.serialize()?;
        fd.write_all(&data)?;
        Ok(())
    }

    pub fn deserialize(&mut self, serialized: &[u8]) -> PyResult<()> {
        let result = self.engine.deserialize(serialized);
        match result {
            Ok(x) => Ok(x),
            Err(error) => {
                let my_blocker_error: BlockerError = error.into();
                Err(my_blocker_error.into())
            }
        }
    }

    pub fn deserialize_from_file(&mut self, file: &str) -> PyResult<()> {
        let mut fd = fs::File::open(file)?;
        let mut data: Vec<u8> = Vec::new();
        fd.read_to_end(&mut data)?;
        self.deserialize(&data)
    }

    pub fn filter_exists(&self, filter: &str) -> bool {
        self.engine.filter_exists(filter)
    }

    pub fn tags_enable(&mut self, tags: Vec<&str>) {
        self.engine.tags_enable(&tags);
    }

    pub fn tags_disable(&mut self, tags: Vec<&str>) {
        self.engine.tags_disable(&tags);
    }

    pub fn tag_exists(&self, tag: &str) -> bool {
        self.engine.tag_exists(tag)
    }

    // pub fn hidden_class_id_selectors(
    //     &self,
    //     classes: Vec<String>,
    //     ids: Vec<String>,
    //     exceptions: &PySet,
    // ) -> PyResult<Vec<String>> {
    //     let mut exception_hashset: HashSet<String> = HashSet::new();
    //     for exception in exceptions.iter() {
    //         let exception_pystr = PyString::from(exception);
    //         exception_hashset.insert(exception.to_string());
    //     }
    //     Ok(self.engine
    //         .hidden_class_id_selectors(&classes, &ids, &exception_hashset))
    // }
}
