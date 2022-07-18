/*! Python `attribute` descriptor class. (PyMethodWrapper)

*/
use super::PyType;
use crate::{
    builtins::{PyStrRef, PyWrapperDescriptor},
    class::PyClassImpl,
    convert::ToPyResult,
    function::{FuncArgs, OwnedParam, RefParam},
    object::PyThreadingConstraint,
    types::{Callable, Constructor, GetDescriptor, Unconstructible},
    AsObject, Context, Py, PyObjectRef, PyPayload, PyRef, PyResult, TryFromObject, VirtualMachine,
};

#[pyclass(module = false, name = "method-wrapper")]
pub struct PyMethodWrapper {
    pub(crate) descr: PyRef<PyWrapperDescriptor>,
    pub(crate) zelf: PyObjectRef,
}

impl std::fmt::Debug for PyMethodWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PyMethodWrapper {{ descr: , zelf: {} }}",
            // self.descr,
            self.zelf,
        )
    }
}

impl PyPayload for PyMethodWrapper {
    fn class(vm: &VirtualMachine) -> &'static Py<PyType> {
        vm.ctx.types.wrapperdescr_type
    }
}

impl PyMethodWrapper {
    pub fn new(descr: PyRef<PyWrapperDescriptor>, zelf: PyObjectRef) -> Self {
        Self { descr, zelf }
    }
}

#[pyimpl(with(Callable, Constructor))]
impl PyMethodWrapper {
    // Descriptor methods
    // #[pymethod(magic)]
    // fn repr(&self) -> String {
    //     format!(
    //         "<method-wrapper '{}' of {} object at {}>",
    //         self.descr.descr_base.name,
    //         self.zelf.class(),
    //         self.zelf
    //     )
    // }

    #[pyproperty(name = "__self__")]
    fn get_self(&self) -> PyObjectRef {
        self.zelf.clone()
    }

    #[pyproperty(magic)]
    fn qualname(&self) -> String {
        format!(
            "{}.{}",
            self.descr.class.slot_name(),
            self.descr.name.clone()
        )
    }

    // #[pymethod(magic)]
    // fn reduce(&self, vm: &VirtualMachine) -> (PyObjectRef, (PyObjectRef, PyStrRef)) {
    //     (
    //         vm.builtins.get_attr("getattr", vm).unwrap(),
    //         (self.zelf, self.descr.descr_base.name.clone()),
    //     )
    // }
    // #[pyproperty(magic)]
    // fn text_signature(&self) -> Option<String> {
    //     self.value.doc.as_ref().and_then(|doc| {
    //         type_::get_text_signature_from_internal_doc(self.value.name.as_str(), doc.as_str())
    //             .map(|signature| signature.to_string())
    //     })
    // }
}
impl Unconstructible for PyMethodWrapper {}

impl Callable for PyMethodWrapper {
    type Args = FuncArgs;
    #[inline]
    fn call(zelf: &crate::Py<Self>, args: FuncArgs, vm: &VirtualMachine) -> PyResult {
        zelf.descr.raw_call(&zelf.zelf, args, vm)
    }
}

pub(crate) fn init(context: &Context) {
    PyMethodWrapper::extend_class(context, context.types.wrapperdescr_type);
}
