/*! Python `attribute` descriptor class. (PyWrapperDescriptor)

*/
use super::PyType;
use crate::{
    builtins::PyMethodWrapper,
    class::PyClassImpl,
    convert::ToPyResult,
    function::{FuncArgs, OwnedParam, RefParam},
    object::PyThreadingConstraint,
    types::{Callable, Constructor, GetDescriptor, Unconstructible, WrapperFunc},
    AsObject, Context, Py, PyObject, PyObjectRef, PyPayload, PyRef, PyResult, TryFromObject,
    VirtualMachine,
};

#[pyclass(module = false, name = "wrapper_descriptor")]
pub struct PyWrapperDescriptor {
    pub(crate) name: String,
    pub(crate) class: &'static Py<PyType>,
    pub(crate) doc: Option<String>,
    pub(crate) wrapper: WrapperFunc,
}

impl std::fmt::Debug for PyWrapperDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PyWrapperDescriptor {{ name: {}, class: {}, doc: {} }}",
            self.name,
            self.class.name(),
            self.doc.as_deref().unwrap_or("None"),
        )
    }
}

impl PyPayload for PyWrapperDescriptor {
    fn class(vm: &VirtualMachine) -> &'static Py<PyType> {
        vm.ctx.types.wrapperdescr_type
    }
}

impl GetDescriptor for PyWrapperDescriptor {
    fn descr_get(
        zelf: PyObjectRef,
        obj: Option<PyObjectRef>,
        _cls: Option<PyObjectRef>,
        vm: &VirtualMachine,
    ) -> PyResult {
        let (zelf, obj) = match Self::_check(zelf, obj, vm) {
            Ok(obj) => obj,
            Err(result) => return result,
        };
        Ok(PyMethodWrapper::new(zelf, obj).into_pyobject(vm))
    }
}

impl PyWrapperDescriptor {
    pub fn new(name: String, class: &'static Py<PyType>, wrapper: WrapperFunc) -> Self {
        Self {
            name,
            class,
            doc: None,
            wrapper,
        }
    }

    pub fn raw_call(&self, zelf: &PyObject, args: FuncArgs, vm: &VirtualMachine) -> PyResult {
        (self.wrapper)(zelf, args, vm)
    }
}

#[pyimpl(with(Callable, GetDescriptor, Constructor))]
impl PyWrapperDescriptor {
    // Descriptor methods

    #[pymethod(magic)]
    fn repr(&self) -> PyResult<String> {
        Ok(format!(
            "<slot wrapper '{}' of '{}' objects>",
            self.name,
            self.class.name()
        ))
    }

    #[pyproperty(magic)]
    fn name(&self) -> String {
        self.name.clone()
    }

    #[pyproperty(magic)]
    fn qualname(&self) -> String {
        format!("{}.{}", self.class.slot_name(), self.name.clone())
    }

    // #[pyproperty(magic)]
    // fn text_signature(&self) -> Option<String> {
    //     self.value.doc.as_ref().and_then(|doc| {
    //         type_::get_text_signature_from_internal_doc(self.value.name.as_str(), doc.as_str())
    //             .map(|signature| signature.to_string())
    //     })
    // }
}

impl Callable for PyWrapperDescriptor {
    type Args = FuncArgs;
    #[inline]
    fn call(zelf: &crate::Py<Self>, args: FuncArgs, vm: &VirtualMachine) -> PyResult {
        if args.args.len() < 1 {
            return Err(vm.new_type_error(format!(
                "descriptor '{}' of '{}' object needs an argument",
                zelf.name(),
                zelf.class.name()
            )));
        }

        let s = args.args[0].clone();
        if !s.is_subclass(zelf.class.as_object(), vm)? {
            return Err(vm.new_type_error(format!(
                "descriptor '{}' requires a '{}' object but received a '{}'",
                zelf.name(),
                zelf.class.name(),
                s.class()
            )));
        }

        zelf.raw_call(s.as_object(), args, vm)
    }
}

impl Unconstructible for PyWrapperDescriptor {}

pub(crate) fn init(context: &Context) {
    PyWrapperDescriptor::extend_class(context, context.types.wrapperdescr_type);
}
