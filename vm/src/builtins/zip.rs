use super::PyTypeRef;
use crate::{
    builtins::IntoPyBool,
    function::{OptionalArg, PosArgs},
    protocol::{PyIter, PyIterReturn},
    slots::{IteratorIterable, SlotConstructor, SlotIterator},
    IntoPyObject, PyClassImpl, PyContext, PyObjectRef, PyRef, PyResult, PyValue, TryFromObject,
    TypeProtocol, VirtualMachine,
};
use crossbeam_utils::atomic::AtomicCell;

#[pyclass(module = false, name = "zip")]
#[derive(Debug)]
pub struct PyZip {
    iterators: Vec<PyIter>,
    strict: AtomicCell<bool>,
}

impl PyValue for PyZip {
    fn class(vm: &VirtualMachine) -> &PyTypeRef {
        &vm.ctx.types.zip_type
    }
}

#[derive(FromArgs)]
pub struct PyZipNewArgs {
    #[pyarg(named, optional)]
    strict: OptionalArg<bool>,
}

impl SlotConstructor for PyZip {
    type Args = (PosArgs<PyIter>, PyZipNewArgs);

    fn py_new(cls: PyTypeRef, (iterators, args): Self::Args, vm: &VirtualMachine) -> PyResult {
        let iterators = iterators.into_vec();
        let strict = AtomicCell::new(args.strict.unwrap_or(false));
        PyZip { iterators, strict }.into_pyresult_with_type(vm, cls)
    }
}

#[pyimpl(with(SlotIterator, SlotConstructor), flags(BASETYPE))]
impl PyZip {
    #[pymethod(magic)]
    fn reduce(zelf: PyRef<Self>, vm: &VirtualMachine) -> PyResult {
        let cls = zelf.clone_class().into_pyobject(vm);
        let iterators = zelf
            .iterators
            .iter()
            .map(|obj| obj.clone().into_object())
            .collect::<Vec<_>>();
        let tupleit = vm.ctx.new_tuple(iterators);
        Ok(if zelf.strict.load() {
            vm.ctx.new_tuple(vec![cls, tupleit, vm.ctx.new_bool(true)])
        } else {
            vm.ctx.new_tuple(vec![cls, tupleit])
        })
    }

    #[pymethod(magic)]
    fn setstate(zelf: PyRef<Self>, state: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
        if let Ok(obj) = IntoPyBool::try_from_object(vm, state) {
            zelf.strict.store(obj.to_bool());
        }
        Ok(())
    }
}

impl IteratorIterable for PyZip {}
impl SlotIterator for PyZip {
    fn next(zelf: &PyRef<Self>, vm: &VirtualMachine) -> PyResult<PyIterReturn> {
        if zelf.iterators.is_empty() {
            return Ok(PyIterReturn::StopIteration(None));
        }
        let mut next_objs = Vec::new();
        for (index, iterator) in zelf.iterators.iter().enumerate() {
            let item = match iterator.next(vm)? {
                PyIterReturn::Return(obj) => obj,
                PyIterReturn::StopIteration(v) => {
                    if zelf.strict.load() {
                        if index > 0 {
                            let plural = if index == 1 { " " } else { "s 1-" };
                            return Err(vm.new_value_error(format!(
                                "zip() argument {} is shorter than argument{}{}",
                                index + 1,
                                plural,
                                index
                            )));
                        }
                        for (index, iterator) in zelf.iterators[1..].iter().enumerate() {
                            let item = match iterator.next(vm)? {
                                PyIterReturn::Return(_obj) => {
                                        let plural = if index == 1 { " " } else { "s 1-" };
                                        return Err(vm.new_value_error(format!(
                                            "zip() argument {} is longer than argument{}{}",
                                            index + 1,
                                            plural,
                                            index
                                        )));
                                },
                                PyIterReturn::StopIteration(v) => return Ok(PyIterReturn::StopIteration(v)),
                            };
                        }
                    }
                    return Ok(PyIterReturn::StopIteration(v));
                }
            };
            next_objs.push(item);
        }
        Ok(PyIterReturn::Return(vm.ctx.new_tuple(next_objs)))
    }
}

pub fn init(context: &PyContext) {
    PyZip::extend_class(context, &context.types.zip_type);
}
