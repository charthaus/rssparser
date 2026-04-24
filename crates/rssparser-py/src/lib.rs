use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

use rssparser_core as core;

#[pyclass(name = "Feed", frozen, module = "rssparser")]
struct PyFeed {
    inner: core::Feed,
}

#[pymethods]
impl PyFeed {
    #[getter]
    fn title(&self) -> Option<&str> {
        self.inner.title.as_deref()
    }
    #[getter]
    fn link(&self) -> Option<&str> {
        self.inner.link.as_deref()
    }
    #[getter]
    fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }
    #[getter]
    fn language(&self) -> Option<&str> {
        self.inner.language.as_deref()
    }
    #[getter]
    fn generator(&self) -> Option<&str> {
        self.inner.generator.as_deref()
    }
    #[getter]
    fn updated(&self) -> Option<&str> {
        self.inner.updated.as_deref()
    }
    #[getter]
    fn id(&self) -> Option<&str> {
        self.inner.id.as_deref()
    }
    #[getter]
    fn icon(&self) -> Option<&str> {
        self.inner.icon.as_deref()
    }
    #[getter]
    fn logo(&self) -> Option<&str> {
        self.inner.logo.as_deref()
    }
    #[getter]
    fn entries<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let items = PyList::empty(py);
        for e in &self.inner.entries {
            items.append(Py::new(py, PyEntry { inner: e.clone() })?)?;
        }
        Ok(items)
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let root = PyDict::new(py);
        root.set_item("feed", feed_to_dict(py, &self.inner)?)?;
        let entries = PyList::empty(py);
        for e in &self.inner.entries {
            entries.append(entry_to_dict(py, e)?)?;
        }
        root.set_item("entries", entries)?;
        Ok(root)
    }

    fn __repr__(&self) -> String {
        format!(
            "Feed(title={:?}, entries={})",
            self.inner.title,
            self.inner.entries.len()
        )
    }
}

#[pyclass(name = "Entry", frozen, module = "rssparser")]
struct PyEntry {
    inner: core::Entry,
}

#[pymethods]
impl PyEntry {
    #[getter]
    fn title(&self) -> Option<&str> {
        self.inner.title.as_deref()
    }
    #[getter]
    fn link(&self) -> Option<&str> {
        self.inner.link.as_deref()
    }
    #[getter]
    fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }
    #[getter]
    fn content(&self) -> Option<&str> {
        self.inner.content.as_deref()
    }
    #[getter]
    fn published(&self) -> Option<&str> {
        self.inner.published.as_deref()
    }
    #[getter]
    fn updated(&self) -> Option<&str> {
        self.inner.updated.as_deref()
    }
    #[getter]
    fn id(&self) -> Option<&str> {
        self.inner.id.as_deref()
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        entry_to_dict(py, &self.inner)
    }

    fn __repr__(&self) -> String {
        format!("Entry(title={:?})", self.inner.title)
    }
}

fn feed_to_dict<'py>(py: Python<'py>, f: &core::Feed) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("title", &f.title)?;
    d.set_item("link", &f.link)?;
    d.set_item("links", links_to_list(py, &f.links)?)?;
    d.set_item("description", &f.description)?;
    d.set_item("language", &f.language)?;
    d.set_item("generator", &f.generator)?;
    d.set_item("updated", &f.updated)?;
    d.set_item("id", &f.id)?;
    d.set_item("image", image_to_dict(py, f.image.as_ref())?)?;
    d.set_item("icon", &f.icon)?;
    d.set_item("logo", &f.logo)?;
    d.set_item("authors", people_to_list(py, &f.authors)?)?;
    d.set_item("categories", categories_to_list(py, &f.categories)?)?;
    Ok(d)
}

fn entry_to_dict<'py>(py: Python<'py>, e: &core::Entry) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("title", &e.title)?;
    d.set_item("link", &e.link)?;
    d.set_item("links", links_to_list(py, &e.links)?)?;
    d.set_item("description", &e.description)?;
    d.set_item("content", &e.content)?;
    d.set_item("published", &e.published)?;
    d.set_item("updated", &e.updated)?;
    d.set_item("id", &e.id)?;
    d.set_item("authors", people_to_list(py, &e.authors)?)?;
    d.set_item("categories", categories_to_list(py, &e.categories)?)?;
    d.set_item("enclosures", enclosures_to_list(py, &e.enclosures)?)?;
    d.set_item("media", media_to_list(py, &e.media)?)?;
    Ok(d)
}

fn links_to_list<'py>(py: Python<'py>, links: &[core::Link]) -> PyResult<Bound<'py, PyList>> {
    let out = PyList::empty(py);
    for l in links {
        let d = PyDict::new(py);
        d.set_item("href", &l.href)?;
        d.set_item("rel", &l.rel)?;
        d.set_item("type", &l.type_)?;
        d.set_item("title", &l.title)?;
        out.append(d)?;
    }
    Ok(out)
}

fn people_to_list<'py>(py: Python<'py>, people: &[core::Person]) -> PyResult<Bound<'py, PyList>> {
    let out = PyList::empty(py);
    for p in people {
        let d = PyDict::new(py);
        d.set_item("name", &p.name)?;
        d.set_item("email", &p.email)?;
        d.set_item("link", &p.link)?;
        out.append(d)?;
    }
    Ok(out)
}

fn categories_to_list<'py>(
    py: Python<'py>,
    cats: &[core::Category],
) -> PyResult<Bound<'py, PyList>> {
    let out = PyList::empty(py);
    for c in cats {
        let d = PyDict::new(py);
        d.set_item("term", &c.term)?;
        d.set_item("scheme", &c.scheme)?;
        d.set_item("label", &c.label)?;
        out.append(d)?;
    }
    Ok(out)
}

fn enclosures_to_list<'py>(
    py: Python<'py>,
    encs: &[core::Enclosure],
) -> PyResult<Bound<'py, PyList>> {
    let out = PyList::empty(py);
    for e in encs {
        let d = PyDict::new(py);
        d.set_item("url", &e.url)?;
        d.set_item("length", &e.length)?;
        d.set_item("type", &e.type_)?;
        out.append(d)?;
    }
    Ok(out)
}

fn media_to_list<'py>(py: Python<'py>, media: &[core::MediaContent]) -> PyResult<Bound<'py, PyList>> {
    let out = PyList::empty(py);
    for m in media {
        let d = PyDict::new(py);
        d.set_item("url", &m.url)?;
        d.set_item("type", &m.type_)?;
        d.set_item("medium", &m.medium)?;
        d.set_item("width", &m.width)?;
        d.set_item("height", &m.height)?;
        d.set_item("duration", &m.duration)?;
        d.set_item("title", &m.title)?;
        d.set_item("description", &m.description)?;
        d.set_item("thumbnail", &m.thumbnail)?;
        out.append(d)?;
    }
    Ok(out)
}

fn image_to_dict<'py>(py: Python<'py>, img: Option<&core::Image>) -> PyResult<Option<Bound<'py, PyDict>>> {
    let Some(img) = img else { return Ok(None) };
    let d = PyDict::new(py);
    d.set_item("url", &img.url)?;
    d.set_item("title", &img.title)?;
    d.set_item("link", &img.link)?;
    d.set_item("width", &img.width)?;
    d.set_item("height", &img.height)?;
    Ok(Some(d))
}

#[pyfunction]
fn parse(data: &[u8]) -> PyResult<PyFeed> {
    // Zero-copy view into Python bytes — GIL held for the duration.
    // Callers wanting true multi-threaded parse should use parse_many, which
    // releases the GIL across the whole batch.
    let feed = core::parse(data).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(PyFeed { inner: feed })
}

#[pyfunction]
fn parse_many<'py>(
    py: Python<'py>,
    blobs: Vec<Bound<'py, PyBytes>>,
) -> PyResult<Bound<'py, PyList>> {
    use rayon::prelude::*;
    // Zero-copy views into Python bytes — valid as long as `blobs` outlives this scope.
    let views: Vec<&[u8]> = blobs.iter().map(|b| b.as_bytes()).collect();

    let results: Vec<Result<core::Feed, String>> = py.allow_threads(|| {
        views
            .into_par_iter()
            .map(|data| core::parse(data).map_err(|e| e.to_string()))
            .collect()
    });

    let out = PyList::empty(py);
    for r in results {
        match r {
            Ok(feed) => out.append(Py::new(py, PyFeed { inner: feed })?)?,
            Err(msg) => return Err(PyValueError::new_err(msg)),
        }
    }
    Ok(out)
}

#[pyfunction]
fn parse_to_json<'py>(py: Python<'py>, data: &[u8]) -> PyResult<Bound<'py, PyBytes>> {
    let feed = core::parse(data).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let bytes = core::feed_to_json_bytes(&feed);
    Ok(PyBytes::new(py, &bytes))
}

#[pyfunction]
fn parse_many_to_json<'py>(
    py: Python<'py>,
    blobs: Vec<Bound<'py, PyBytes>>,
) -> PyResult<Bound<'py, PyList>> {
    use rayon::prelude::*;
    let views: Vec<&[u8]> = blobs.iter().map(|b| b.as_bytes()).collect();

    let results: Vec<Result<Vec<u8>, String>> = py.allow_threads(|| {
        views
            .into_par_iter()
            .map(|data| {
                core::parse(data)
                    .map(|f| core::feed_to_json_bytes(&f))
                    .map_err(|e| e.to_string())
            })
            .collect()
    });

    let out = PyList::empty(py);
    for r in results {
        match r {
            Ok(json) => out.append(PyBytes::new(py, &json))?,
            Err(msg) => return Err(PyValueError::new_err(msg)),
        }
    }
    Ok(out)
}

#[pymodule]
fn _rssparser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFeed>()?;
    m.add_class::<PyEntry>()?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(parse_many, m)?)?;
    m.add_function(wrap_pyfunction!(parse_to_json, m)?)?;
    m.add_function(wrap_pyfunction!(parse_many_to_json, m)?)?;
    Ok(())
}
