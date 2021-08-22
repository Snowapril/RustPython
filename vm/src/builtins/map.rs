use super::pytype::PyTypeRef;
use crate::function::Args;
use crate::iterator;
use crate::slots::PyIter;
use crate::vm::VirtualMachine;
use crate::pyobject::TypeProtocol;
use crate::{PyClassImpl, PyContext, PyObjectRef, PyRef, PyResult, PyValue, TryFromObject};

/// map(func, *iterables) --> map object
///
/// Make an iterator that computes the function using arguments from
/// each of the iterables.  Stops when the shortest iterable is exhausted.
#[pyclass(module = false, name = "map")]
#[derive(Debug)]
pub struct PyMap {
    mapper: PyObjectRef,
    iterators: Vec<PyObjectRef>,
}

impl PyValue for PyMap {
    fn class(vm: &VirtualMachine) -> &PyTypeRef {
        &vm.ctx.types.map_type
    }
}

#[pyimpl(with(PyIter), flags(BASETYPE))]
impl PyMap {
    #[pyslot]
    fn tp_new(
        cls: PyTypeRef,
        function: PyObjectRef,
        iterables: Args,
        vm: &VirtualMachine,
    ) -> PyResult<PyRef<Self>> {
        let iterators = iterables
            .into_iter()
            .map(|iterable| iterator::get_iter(vm, iterable))
            .collect::<Result<Vec<_>, _>>()?;
        PyMap {
            mapper: function,
            iterators,
        }
        .into_ref_with_type(vm, cls)
    }

    #[pymethod(magic)]
    fn length_hint(&self, vm: &VirtualMachine) -> PyResult<usize> {
        self.iterators.iter().try_fold(0, |prev, cur| {
            let cur = iterator::length_hint(vm, cur.clone())?.unwrap_or(0);
            let max = std::cmp::max(prev, cur);
            Ok(max)
        })
    }
}

impl PyIter for PyMap {
    fn next(zelf: &PyRef<Self>, vm: &VirtualMachine) -> PyResult {
        let next_objs = zelf
            .iterators
            .iter()
            .map(|iterator| iterator::call_next(vm, iterator))
            .collect::<Result<Vec<_>, _>>()?;

        // the mapper itself can raise StopIteration which does stop the map iteration
        vm.invoke(&zelf.mapper, next_objs)
    }
}

pub fn init(context: &PyContext) {
    PyMap::extend_class(context, &context.types.map_type);
}

#[derive(Default)]
pub struct PyMappingMethods {
    pub length: Option<fn(PyObjectRef, &VirtualMachine) -> PyResult<usize>>,
    pub subscript: Option<fn(PyObjectRef, PyObjectRef, &VirtualMachine) -> PyResult>,
    pub ass_subscript: Option<fn(PyObjectRef, &PyObjectRef, &PyObjectRef, &VirtualMachine) -> PyResult<()>>,
}

pub trait PyMappingProtocol: PyValue {
    fn length(map: PyObjectRef, vm: &VirtualMachine) -> PyResult<usize>;
    fn subscript(map: PyObjectRef, needle: PyObjectRef, vm: &VirtualMachine) -> PyResult;
    fn ass_subscript(map: PyObjectRef, index: &PyObjectRef, values: &PyObjectRef, vm: &VirtualMachine) -> PyResult<()>;
}

impl TryFromObject for PyMappingMethods {
    fn try_from_object(vm: &VirtualMachine, obj: PyObjectRef) -> PyResult<Self> {
        let obj_cls = obj.class();
        for cls in obj_cls.iter_mro() {
            if let Some(f) = cls.slots.as_mapping.as_ref() {
                return f(&obj, vm);
            }
        }
        Err(vm.new_type_error(format!(
            "a dict-like object is required, not '{}'",
            obj_cls.name
        )))
    }
}