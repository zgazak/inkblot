use pyo3::prelude::*;
use pyo3::types::PyDict;

mod colors;
mod layout;
mod render;
mod text;

/// Render a plot specification to PNG bytes.
#[pyfunction]
fn render_plot(spec: &Bound<'_, PyDict>) -> PyResult<Vec<u8>> {
    let plot_spec = render::PlotSpec::from_pydict(spec)?;
    render::render_plot(&plot_spec)
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(render_plot, m)?)?;
    Ok(())
}
