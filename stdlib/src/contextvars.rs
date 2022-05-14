pub(crate) use _contextvars::make_module;

#[pymodule]
mod _contextvars {
    use crate::vm::{
        builtins::{PyFunction, PyGenericAlias, PyStrRef, PyTypeRef},
        common::hash::PyHash,
        function::{ArgCallable, FuncArgs, OptionalArg},
        types::{Constructor, Hashable, Initializer},
        AsObject, Py, PyObjectRef, PyPayload, PyRef, PyResult, VirtualMachine,
    };

    #[pyattr]
    #[pyclass(name = "Context")]
    #[derive(Debug, PyPayload)]
    struct PyContext {} // not to confuse with vm::Context

    #[pyimpl(with(Initializer))]
    impl PyContext {
        #[pymethod]
        fn run(
            &self,
            _callable: ArgCallable,
            _args: FuncArgs,
            _vm: &VirtualMachine,
        ) -> PyResult<PyFunction> {
            unimplemented!("Context.run is currently under construction")
        }

        #[pymethod]
        fn copy(&self, _vm: &VirtualMachine) -> PyResult<Self> {
            unimplemented!("Context.copy is currently under construction")
        }

        #[pymethod(magic)]
        fn getitem(&self, _var: PyObjectRef) -> PyResult<PyObjectRef> {
            unimplemented!("Context.__getitem__ is currently under construction")
        }

        #[pymethod(magic)]
        fn contains(&self, _var: PyObjectRef) -> PyResult<bool> {
            unimplemented!("Context.__contains__ is currently under construction")
        }

        #[pymethod(magic)]
        fn len(&self) -> usize {
            unimplemented!("Context.__len__ is currently under construction")
        }

        #[pymethod(magic)]
        fn iter(&self) -> PyResult {
            unimplemented!("Context.__iter__ is currently under construction")
        }

        #[pymethod]
        fn get(
            &self,
            _key: PyObjectRef,
            _default: OptionalArg<PyObjectRef>,
        ) -> PyResult<PyObjectRef> {
            unimplemented!("Context.get is currently under construction")
        }

        #[pymethod]
        fn keys(_zelf: PyRef<Self>, _vm: &VirtualMachine) -> Vec<PyObjectRef> {
            unimplemented!("Context.keys is currently under construction")
        }

        #[pymethod]
        fn values(_zelf: PyRef<Self>, _vm: &VirtualMachine) -> Vec<PyObjectRef> {
            unimplemented!("Context.values is currently under construction")
        }
    }

    impl Initializer for PyContext {
        type Args = FuncArgs;

        fn init(_obj: PyRef<Self>, _args: Self::Args, _vm: &VirtualMachine) -> PyResult<()> {
            unimplemented!("Context.__init__ is currently under construction")
        }
    }

    #[pyattr]
    #[pyclass(name)]
    #[derive(Debug, PyPayload)]
    struct ContextVar {
        #[allow(dead_code)] // TODO: RUSTPYTHON
        name: PyStrRef,
        #[allow(dead_code)] // TODO: RUSTPYTHON
        default: Option<PyObjectRef>,
        cached: Option<PyObjectRef>,
        cached_tsid: u64,
        cached_tsver: u64,
    }

    #[derive(FromArgs)]
    struct ContextVarOptions {
        #[pyarg(positional)]
        #[allow(dead_code)] // TODO: RUSTPYTHON
        name: PyStrRef,
        #[pyarg(any, optional)]
        #[allow(dead_code)] // TODO: RUSTPYTHON
        default: OptionalArg<PyObjectRef>,
    }

    #[pyimpl(with(Hashable, Constructor))]
    impl ContextVar {
        #[pyproperty]
        fn name(&self) -> PyStrRef {
            self.name.clone()
        }

        #[pymethod]
        fn get(
            &self,
            _default: OptionalArg<PyObjectRef>,
            _vm: &VirtualMachine,
        ) -> PyResult<PyObjectRef> {
            unimplemented!("ContextVar.get() is currently under construction")
        }

        #[pymethod]
        fn set(&self, _value: PyObjectRef, _vm: &VirtualMachine) -> PyResult<()> {
            unimplemented!("ContextVar.set() is currently under construction")
        }

        #[pymethod]
        fn reset(
            _zelf: PyRef<Self>,
            _token: PyRef<ContextToken>,
            _vm: &VirtualMachine,
        ) -> PyResult<()> {
            unimplemented!("ContextVar.reset() is currently under construction")
        }

        #[pyclassmethod(magic)]
        fn class_getitem(cls: PyTypeRef, args: PyObjectRef, vm: &VirtualMachine) -> PyGenericAlias {
            PyGenericAlias::new(cls, args, vm)
        }

        #[pymethod(magic)]
        fn repr(zelf: PyRef<Self>, vm: &VirtualMachine) -> PyResult<String> {
            Ok(if let Some(default) = zelf.default.clone() {
                format!(
                    "<ContextVar name={} default={} at {:#x}>",
                    zelf.name.as_object().repr(vm)?,
                    default.repr(vm)?,
                    zelf.get_id()
                )
            } else {
                format!(
                    "<ContextVar name={} at {:#x}>",
                    zelf.name.as_object().repr(vm)?,
                    zelf.get_id()
                )
            })
        }
    }

    impl Constructor for ContextVar {
        type Args = ContextVarOptions;

        fn py_new(cls: PyTypeRef, args: Self::Args, vm: &VirtualMachine) -> PyResult {
            ContextVar {
                name: args.name,
                default: args.default.into_option(),
                cached: None,
                cached_tsid: 0u64,
                cached_tsver: 0u64,
            }
            .into_ref_with_type(vm, cls)
            .map(Into::into)
        }
    }

    impl Hashable for ContextVar {
        #[inline]
        fn hash(zelf: &Py<Self>, vm: &VirtualMachine) -> PyResult<PyHash> {
            let name_hash = zelf.name.as_object().hash(vm)?;
            Ok(zelf.get_id() as i64 ^ name_hash)
        }
    }

    #[pyattr]
    #[pyclass(name = "Token")]
    #[derive(Debug, PyPayload)]
    struct ContextToken {}

    #[derive(FromArgs)]
    struct ContextTokenOptions {
        #[pyarg(positional)]
        #[allow(dead_code)] // TODO: RUSTPYTHON
        context: PyObjectRef,
        #[pyarg(positional)]
        #[allow(dead_code)] // TODO: RUSTPYTHON
        var: PyObjectRef,
        #[pyarg(positional)]
        #[allow(dead_code)] // TODO: RUSTPYTHON
        old_value: PyObjectRef,
    }

    #[pyimpl(with(Initializer))]
    impl ContextToken {
        #[pyproperty]
        fn var(&self, _vm: &VirtualMachine) -> PyObjectRef {
            unimplemented!("Token.var() is currently under construction")
        }

        #[pyproperty]
        fn old_value(&self, _vm: &VirtualMachine) -> PyObjectRef {
            unimplemented!("Token.old_value() is currently under construction")
        }

        #[pymethod(magic)]
        fn repr(_zelf: PyRef<Self>, _vm: &VirtualMachine) -> String {
            unimplemented!("<Token {{}}var={{}} at {{}}>")
        }
    }

    impl Initializer for ContextToken {
        type Args = ContextTokenOptions;

        fn init(_obj: PyRef<Self>, _args: Self::Args, _vm: &VirtualMachine) -> PyResult<()> {
            unimplemented!("Token.__init__() is currently under construction")
        }
    }

    #[pyfunction]
    fn copy_context() {}
}
