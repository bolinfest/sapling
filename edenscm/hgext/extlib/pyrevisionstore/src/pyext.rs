// Copyright Facebook, Inc. 2018
//! Python bindings for a Rust hg store

use std::{
    cell::{Ref, RefCell},
    path::PathBuf,
};

use cpython::{
    ObjectProtocol, PyBytes, PyClone, PyDict, PyList, PyObject, PyResult, PyTuple, Python,
    PythonObject,
};

use encoding;
use failure::{format_err, Error};
use revisionstore::{
    repack::{filter_incrementalpacks, list_packs, repack_datapacks, repack_historypacks},
    DataPack, HistoryPack,
};

use crate::datastorepyext::DataStorePyExt;
use crate::historystorepyext::HistoryStorePyExt;
use crate::pythondatastore::PythonDataStore;
use crate::pythonutil::to_pyerr;
use crate::repackablepyext::RepackablePyExt;

py_module_initializer!(
    pyrevisionstore,        // module name
    initpyrevisionstore,    // py2 init name
    PyInit_pyrevisionstore, // py3 init name
    |py, m| {
        // init function
        m.add_class::<datastore>(py)?;
        m.add_class::<datapack>(py)?;
        m.add_class::<historypack>(py)?;
        m.add(
            py,
            "repackdatapacks",
            py_fn!(py, repackdata(packpath: PyBytes, outdir: PyBytes)),
        )?;
        m.add(
            py,
            "repackincrementaldatapacks",
            py_fn!(
                py,
                incremental_repackdata(packpath: PyBytes, outdir: PyBytes)
            ),
        )?;
        m.add(
            py,
            "repackhistpacks",
            py_fn!(py, repackhist(packpath: PyBytes, outdir: PyBytes)),
        )?;
        m.add(
            py,
            "repackincrementalhistpacks",
            py_fn!(
                py,
                incremental_repackhist(packpath: PyBytes, outdir: PyBytes)
            ),
        )?;
        Ok(())
    }
);

/// Helper function to de-serialize and re-serialize from and to Python objects.
fn repack_pywrapper(
    py: Python,
    packpath: PyBytes,
    outdir_py: PyBytes,
    repacker: impl FnOnce(PathBuf, PathBuf) -> Result<PathBuf, Error>,
) -> PyResult<PyBytes> {
    let path =
        encoding::local_bytes_to_path(packpath.data(py)).map_err(|e| to_pyerr(py, &e.into()))?;

    let outdir =
        encoding::local_bytes_to_path(outdir_py.data(py)).map_err(|e| to_pyerr(py, &e.into()))?;
    repacker(path.to_path_buf(), outdir.to_path_buf())
        .and_then(|p| Ok(PyBytes::new(py, &encoding::path_to_local_bytes(&p)?)))
        .map_err(|e| to_pyerr(py, &e.into()))
}

/// Merge all the datapacks into one big datapack. Returns the fullpath of the resulting datapack.
fn repackdata(py: Python, packpath: PyBytes, outdir_py: PyBytes) -> PyResult<PyBytes> {
    repack_pywrapper(py, packpath, outdir_py, |dir, outdir| {
        repack_datapacks(list_packs(&dir, "datapack")?.iter(), &outdir)
    })
}

/// Merge all the history packs into one big historypack. Returns the fullpath of the resulting
/// histpack.
fn repackhist(py: Python, packpath: PyBytes, outdir_py: PyBytes) -> PyResult<PyBytes> {
    repack_pywrapper(py, packpath, outdir_py, |dir, outdir| {
        repack_historypacks(list_packs(&dir, "histpack")?.iter(), &outdir)
    })
}

/// Perform an incremental repack of data packs.
fn incremental_repackdata(py: Python, packpath: PyBytes, outdir_py: PyBytes) -> PyResult<PyBytes> {
    repack_pywrapper(py, packpath, outdir_py, |dir, outdir| {
        repack_datapacks(
            filter_incrementalpacks(list_packs(&dir, "datapack")?, "datapack")?.iter(),
            &outdir,
        )
    })
}

/// Perform an incremental repack of history packs.
fn incremental_repackhist(py: Python, packpath: PyBytes, outdir_py: PyBytes) -> PyResult<PyBytes> {
    repack_pywrapper(py, packpath, outdir_py, |dir, outdir| {
        repack_historypacks(
            filter_incrementalpacks(list_packs(&dir, "histpack")?, "histpack")?.iter(),
            &outdir,
        )
    })
}

py_class!(class datastore |py| {
    data store: Box<DataStorePyExt + Send>;

    def __new__(
        _cls,
        store: &PyObject
    ) -> PyResult<datastore> {
        datastore::create_instance(
            py,
            Box::new(PythonDataStore::new(store.clone_ref(py))),
        )
    }

    def get(&self, name: &PyBytes, node: &PyBytes) -> PyResult<PyBytes> {
        self.store(py).get(py, name, node)
    }

    def getdeltachain(&self, name: &PyBytes, node: &PyBytes) -> PyResult<PyList> {
        self.store(py).get_delta_chain(py, name, node)
    }

    def getmeta(&self, name: &PyBytes, node: &PyBytes) -> PyResult<PyDict> {
        self.store(py).get_meta(py, name, node)
    }

    def getmissing(&self, keys: &PyObject) -> PyResult<PyList> {
        self.store(py).get_missing(py, &mut keys.iter(py)?)
    }
});

/// The cpython crates forces us to use a `RefCell` for mutation, `OptionalRefCell` wraps all the logic
/// of dealing with it.
struct OptionalRefCell<T> {
    inner: RefCell<Option<T>>,
}

impl<T> OptionalRefCell<T> {
    pub fn new(value: T) -> OptionalRefCell<T> {
        OptionalRefCell {
            inner: RefCell::new(Some(value)),
        }
    }

    /// Obtain a reference on the stored value. Will fail if the value was previously consumed.
    pub fn get_value(&self) -> Result<Ref<T>, failure::Error> {
        let b = self.inner.borrow();
        if b.as_ref().is_none() {
            Err(format_err!("OptionalRefCell is None."))
        } else {
            Ok(Ref::map(b, |o| o.as_ref().unwrap()))
        }
    }

    /// Consume the stored value and returns it. Will fail if the value was previously consumed.
    pub fn take_value(&self) -> Result<T, failure::Error> {
        let opt = self.inner.try_borrow_mut()?.take();
        opt.ok_or_else(|| format_err!("None"))
    }
}

/// Wrapper around `OptionalRefCell<T>` to convert from `Result<T>` to `PyResult<T>`
struct PyOptionalRefCell<T> {
    inner: OptionalRefCell<T>,
}

impl<T> PyOptionalRefCell<T> {
    pub fn new(value: T) -> PyOptionalRefCell<T> {
        PyOptionalRefCell {
            inner: OptionalRefCell::new(value),
        }
    }

    pub fn get_value(&self, py: Python) -> PyResult<Ref<T>> {
        self.inner.get_value().map_err(|e| to_pyerr(py, &e.into()))
    }

    pub fn take_value(&self, py: Python) -> PyResult<T> {
        self.inner.take_value().map_err(|e| to_pyerr(py, &e.into()))
    }
}

py_class!(class datapack |py| {
    data store: PyOptionalRefCell<Box<DataPack>>;

    def __new__(
        _cls,
        path: &PyBytes
    ) -> PyResult<datapack> {
        let path = encoding::local_bytes_to_path(path.data(py))
                                 .map_err(|e| to_pyerr(py, &e.into()))?;
        datapack::create_instance(
            py,
            PyOptionalRefCell::new(Box::new(match DataPack::new(&path) {
                Ok(pack) => pack,
                Err(e) => return Err(to_pyerr(py, &e)),
            })),
        )
    }

    def path(&self) -> PyResult<PyBytes> {
        let store = self.store(py).get_value(py)?;
        let path = encoding::path_to_local_bytes(store.base_path()).map_err(|e| to_pyerr(py, &e.into()))?;
        Ok(PyBytes::new(py, &path))
    }

    def packpath(&self) -> PyResult<PyBytes> {
        let store = self.store(py).get_value(py)?;
        let path = encoding::path_to_local_bytes(store.pack_path()).map_err(|e| to_pyerr(py, &e.into()))?;
        Ok(PyBytes::new(py, &path))
    }

    def indexpath(&self) -> PyResult<PyBytes> {
        let store = self.store(py).get_value(py)?;
        let path = encoding::path_to_local_bytes(store.index_path()).map_err(|e| to_pyerr(py, &e.into()))?;
        Ok(PyBytes::new(py, &path))
    }

    def get(&self, name: &PyBytes, node: &PyBytes) -> PyResult<PyBytes> {
        let store = self.store(py).get_value(py)?;
        store.get(py, name, node)
    }

    def getdelta(&self, name: &PyBytes, node: &PyBytes) -> PyResult<PyObject> {
        let store = self.store(py).get_value(py)?;
        store.get_delta(py, name, node)
    }

    def getdeltachain(&self, name: &PyBytes, node: &PyBytes) -> PyResult<PyList> {
        let store = self.store(py).get_value(py)?;
        store.get_delta_chain(py, name, node)
    }

    def getmeta(&self, name: &PyBytes, node: &PyBytes) -> PyResult<PyDict> {
        let store = self.store(py).get_value(py)?;
        store.get_meta(py, name, node)
    }

    def getmissing(&self, keys: &PyObject) -> PyResult<PyList> {
        let store = self.store(py).get_value(py)?;
        store.get_missing(py, &mut keys.iter(py)?)
    }

    def markledger(&self, ledger: &PyObject, _options: &PyObject) -> PyResult<PyObject> {
        let store = self.store(py).get_value(py)?;
        store.mark_ledger(py, self.as_object(), ledger)?;
        Ok(Python::None(py))
    }

    def cleanup(&self, ledger: &PyObject) -> PyResult<PyObject> {
        let datapack = self.store(py).take_value(py)?;
        datapack.cleanup(py, ledger)?;
        Ok(Python::None(py))
    }
});

py_class!(class historypack |py| {
    data store: PyOptionalRefCell<Box<HistoryPack>>;

    def __new__(
        _cls,
        path: &PyBytes
    ) -> PyResult<historypack> {
        let path = encoding::local_bytes_to_path(path.data(py))
                                 .map_err(|e| to_pyerr(py, &e.into()))?;
        historypack::create_instance(
            py,
            PyOptionalRefCell::new(Box::new(match HistoryPack::new(&path) {
                Ok(pack) => pack,
                Err(e) => return Err(to_pyerr(py, &e)),
            })),
        )
    }

    def path(&self) -> PyResult<PyBytes> {
        let store = self.store(py).get_value(py)?;
        let path = encoding::path_to_local_bytes(store.base_path()).map_err(|e| to_pyerr(py, &e.into()))?;
        Ok(PyBytes::new(py, &path))
    }

    def packpath(&self) -> PyResult<PyBytes> {
        let store = self.store(py).get_value(py)?;
        let path = encoding::path_to_local_bytes(store.pack_path()).map_err(|e| to_pyerr(py, &e.into()))?;
        Ok(PyBytes::new(py, &path))
    }

    def indexpath(&self) -> PyResult<PyBytes> {
        let store = self.store(py).get_value(py)?;
        let path = encoding::path_to_local_bytes(store.index_path()).map_err(|e| to_pyerr(py, &e.into()))?;
        Ok(PyBytes::new(py, &path))
    }

    def getancestors(&self, name: &PyBytes, node: &PyBytes, known: Option<&PyObject>) -> PyResult<PyDict> {
        let _known = known;
        let store = self.store(py).get_value(py)?;
        store.get_ancestors(py, name, node)
    }

    def getmissing(&self, keys: &PyObject) -> PyResult<PyList> {
        let store = self.store(py).get_value(py)?;
        store.get_missing(py, &mut keys.iter(py)?)
    }

    def getnodeinfo(&self, name: &PyBytes, node: &PyBytes) -> PyResult<PyTuple> {
        let store = self.store(py).get_value(py)?;
        store.get_node_info(py, name, node)
    }

    def markledger(&self, ledger: &PyObject, _options: &PyObject) -> PyResult<PyObject> {
        let store = self.store(py).get_value(py)?;
        store.mark_ledger(py, self.as_object(), ledger)?;
        Ok(Python::None(py))
    }

    def cleanup(&self, ledger: &PyObject) -> PyResult<PyObject> {
        let historypack = self.store(py).take_value(py)?;
        historypack.cleanup(py, ledger)?;
        Ok(Python::None(py))
    }
});
