use crate::common::lock::PyRwLock;

use crossbeam_utils::atomic::AtomicCell;
use num_bigint::BigInt;
use num_traits::Zero;

use super::int::{try_to_primitive, PyInt, PyIntRef};
use super::iter::{
    IterStatus,
    IterStatus::{Active, Exhausted},
};
use indexmap::IndexMap;
use super::pytype::{PyTypeRef, DerefToPyType};
use crate::function::{OptionalArg, FuncArgs};
use crate::slots::PyIter;
use crate::vm::VirtualMachine;
use crate::{iterator, ItemProtocol, TypeProtocol};
use crate::{IntoPyObject, PyClassImpl, PyContext, PyObjectRef, PyRef, PyResult, PyValue};

#[pyclass(module = false, name = "enumerate")]
#[derive(Debug)]
pub struct PyEnumerate {
    counter: PyRwLock<BigInt>,
    iterator: PyObjectRef,
}

impl PyValue for PyEnumerate {
    fn class(vm: &VirtualMachine) -> &PyTypeRef {
        &vm.ctx.types.enumerate_type
    }
}

#[derive(FromArgs)]
struct EnumerateArgs {
    #[pyarg(any)]
    iterable: PyObjectRef,
    #[pyarg(any, optional)]
    start: OptionalArg<PyIntRef>,
}

#[pyimpl(with(PyIter), flags(BASETYPE))]
impl PyEnumerate {
    #[pyslot]
    fn tp_new(cls: PyTypeRef, args: EnumerateArgs, vm: &VirtualMachine) -> PyResult<PyRef<Self>> {
        let counter = match args.start {
            OptionalArg::Present(start) => start.as_bigint().clone(),
            OptionalArg::Missing => BigInt::zero(),
        };

        let iterator = iterator::get_iter(vm, args.iterable)?;
        PyEnumerate {
            counter: PyRwLock::new(counter),
            iterator,
        }
        .into_ref_with_type(vm, cls)
    }
}

impl PyIter for PyEnumerate {
    fn next(zelf: &PyRef<Self>, vm: &VirtualMachine) -> PyResult {
        let next_obj = iterator::call_next(vm, &zelf.iterator)?;
        let mut counter = zelf.counter.write();
        let position = counter.clone();
        *counter += 1;
        Ok((position, next_obj).into_pyobject(vm))
    }
}

#[pyclass(module = false, name = "reversed")]
#[derive(Debug)]
pub struct PyReverseSequenceIterator {
    pub position: AtomicCell<isize>,
    pub status: AtomicCell<IterStatus>,
    pub obj: PyObjectRef,
}

impl PyValue for PyReverseSequenceIterator {
    fn class(vm: &VirtualMachine) -> &PyTypeRef {
        &vm.ctx.types.reverse_iter_type
    }
}

#[pyimpl(with(PyIter))]
impl PyReverseSequenceIterator {
    pub fn new(obj: PyObjectRef, len: isize) -> Self {
        Self {
            position: AtomicCell::new(len - 1),
            status: AtomicCell::new(if len == 0 {
                Exhausted
            } else {
                Active
            }),
            obj,
        }
    }

    #[pymethod(magic)]
    fn length_hint(&self) -> PyResult<isize> {
        Ok(self.position.load() + 1)
    }

    #[pymethod(magic)]
    fn setstate(&self, state: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
        // When we're exhausted, just return.
        if let Exhausted = self.status.load() {
            return Ok(());
        }
        let len = vm.obj_len_opt(&self.obj).unwrap()? as isize; // TODO: unwrap을 좀더 나은방향으로 바꾸자 unwrap은 panic을 유발함
        let pos = state
            .payload::<PyInt>()
            .ok_or_else(|| vm.new_type_error("an integer is required.".to_owned()))?;
        let pos = std::cmp::min(
            try_to_primitive(pos.as_bigint(), vm).unwrap_or(0),
            len - 1
        );
        self.position.store(pos);
        Ok(())
    }

    #[pymethod(magic)]
    fn reduce(&self, vm: &VirtualMachine) -> PyResult {
        let iter = vm.get_attribute(vm.builtins.clone(), "reversed")?;
        let obj_type = self.obj.clone_class().deref_to_type();
        obj_type.slots.new.map(|new| {
            new(vm, FuncArgs {
                args: Vec::new(),
                kwargs: IndexMap::new(),
            })
        });
        Ok(vm.ctx.new_tuple(match self.status.load() {
            Exhausted => vec![iter, vm.ctx.new_tuple(vec![])], // self.obj와 같은 타입의 empty object를 생성하는 방법을 모르겠네
            Active => vec![
                iter,
                vm.ctx.new_tuple(vec![self.obj.clone()]),
                vm.ctx
                    .new_int(self.position.load()),
            ],
        }))
    }
}

impl PyIter for PyReverseSequenceIterator {
    fn next(zelf: &PyRef<Self>, vm: &VirtualMachine) -> PyResult {
        if let Exhausted = zelf.status.load() {
            return Err(vm.new_stop_iteration());
        }
        let pos = zelf.position.fetch_sub(1);
        if pos >= 0 {
            match zelf.obj.get_item(pos, vm) {
                Err(ref e) if e.isinstance(&vm.ctx.exceptions.index_error) => {
                    zelf.status.store(Exhausted);
                    Err(vm.new_stop_iteration())
                }
                // also catches stop_iteration => stop_iteration
                ret => ret,
            }
        } else {
            zelf.status.store(Exhausted);
            Err(vm.new_stop_iteration())
        }
    }
}

pub fn init(context: &PyContext) {
    PyEnumerate::extend_class(context, &context.types.enumerate_type);
    PyReverseSequenceIterator::extend_class(context, &context.types.reverse_iter_type);
}
