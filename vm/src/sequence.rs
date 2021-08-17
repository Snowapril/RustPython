use std::{fmt::Debug, ops::Deref};

use crate::builtins::dict::{PyMapping};
use crate::common::rc::PyRc;
use crate::slots::{PyComparisonOp};
use crate::{TryFromBorrowedObject, VirtualMachine};
use crate::{PyObjectRef, PyResult, TypeProtocol};
use num_traits::cast::ToPrimitive;

pub(super) type DynPyIter<'a> = Box<dyn ExactSizeIterator<Item = &'a PyObjectRef> + 'a>;

#[allow(clippy::len_without_is_empty)]
pub(crate) trait SimpleSeq {
    fn len(&self) -> usize;
    fn boxed_iter(&self) -> DynPyIter;
}

impl<'a, D> SimpleSeq for D
where
    D: 'a + std::ops::Deref<Target = [PyObjectRef]>,
{
    fn len(&self) -> usize {
        self.deref().len()
    }

    fn boxed_iter(&self) -> DynPyIter {
        Box::new(self.deref().iter())
    }
}

pub(crate) fn eq(vm: &VirtualMachine, zelf: DynPyIter, other: DynPyIter) -> PyResult<bool> {
    if zelf.len() == other.len() {
        for (a, b) in Iterator::zip(zelf, other) {
            if !vm.identical_or_equal(a, b)? {
                return Ok(false);
            }
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn cmp(
    vm: &VirtualMachine,
    zelf: DynPyIter,
    other: DynPyIter,
    op: PyComparisonOp,
) -> PyResult<bool> {
    let less = match op {
        PyComparisonOp::Eq => return eq(vm, zelf, other),
        PyComparisonOp::Ne => return eq(vm, zelf, other).map(|eq| !eq),
        PyComparisonOp::Lt | PyComparisonOp::Le => true,
        PyComparisonOp::Gt | PyComparisonOp::Ge => false,
    };
    let (lhs_len, rhs_len) = (zelf.len(), other.len());
    for (a, b) in Iterator::zip(zelf, other) {
        let ret = if less {
            vm.bool_seq_lt(a, b)?
        } else {
            vm.bool_seq_gt(a, b)?
        };
        if let Some(v) = ret {
            return Ok(v);
        }
    }
    Ok(op.eval_ord(lhs_len.cmp(&rhs_len)))
}

pub(crate) struct SeqMul<'a> {
    seq: &'a dyn SimpleSeq,
    repetitions: usize,
    iter: Option<DynPyIter<'a>>,
}

impl ExactSizeIterator for SeqMul<'_> {}

impl<'a> Iterator for SeqMul<'a> {
    type Item = &'a PyObjectRef;
    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.as_mut().and_then(Iterator::next) {
            Some(item) => Some(item),
            None => {
                if self.repetitions == 0 {
                    None
                } else {
                    self.repetitions -= 1;
                    self.iter = Some(self.seq.boxed_iter());
                    self.next()
                }
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.iter.as_ref().map_or(0, ExactSizeIterator::len)
            + (self.repetitions * self.seq.len());
        (size, Some(size))
    }
}

pub(crate) fn seq_mul(seq: &impl SimpleSeq, repetitions: isize) -> SeqMul {
    let repetitions = if seq.len() > 0 {
        repetitions.to_usize().unwrap_or(0)
    } else {
        0
    };
    SeqMul {
        seq,
        repetitions,
        iter: None,
    }
}

pub trait PySequenceMethods: Debug {
    fn length(&self) -> PyResult<usize>;
    
    fn size(&self) -> PyResult<usize> {
        return self.length()
    }
    
    fn concat(&self, other: PyObjectRef, vm: &VirtualMachine) -> PyResult;
    
    fn repeat(&self, count: usize, vm: &VirtualMachine) -> PyResult;
    
    fn concat_inplace(&self, other: PyObjectRef, vm: &VirtualMachine) -> PyObjectRef;
    
    fn repeat_inplace(&self, other: PyObjectRef, vm: &VirtualMachine) -> PyObjectRef;
    
    fn get_slice(&self, i1: isize, i2: isize, vm: &VirtualMachine) -> PyResult;
    
    fn set_slice(&self, slice: PySliceRef, sec: PyIterable, vm: &VirtualMachine) -> PyResult<()>;
    
    fn del_slice(&self, i1: isize, i2: isize, vm: &VirtualMachine) -> PyResult<()>;

    fn get_item(&self, i: isize, vm: &VirtualMachine) -> PyResult;

    fn set_item(&self, i: isize, v: PyObjectRef, vm: &VirtualMachine) -> PyResult<()>;
    
    fn del_item(&self, i: isize, vm: &VirtualMachine) -> PyResult<()>;
    
    fn count(&self, value: PyObjectRef, vm: &VirtualMachine) -> PyResult<usize>;
    
    fn contains(&self, value: PyObjectRef, vm: &VirtualMachine) -> PyResult<bool>;
    
    fn index(&self, value: PyObjectRef, vm: &VirtualMachine) -> PyResult<usize>;
    
    fn to_vec(&self, vm: &VirtualMachine) -> PyResult<Vec<PyObjectRef>>;

    fn list(&self, vm: &VirtualMachine) -> PyResult {
        Ok(vm.ctx.new_list(self.to_vec(vm)?))
    }

    fn tuple(&self, vm: &VirtualMachine) -> PyResult {
        Ok(vm.ctx.new_tuple(self.to_vec(vm)?))
    }
}

#[derive(Debug)]
pub struct PySequenceMethodsRef(Box<dyn PySequenceMethods>);

impl TryFromBorrowedObject for PySequenceMethodsRef {
    fn try_from_borrowed_object(vm: &VirtualMachine, obj: &PyObjectRef) -> PyResult<Self> {
        let obj_cls = obj.class();
        if let Err(_) = PyMapping::try_from_object(vm, obj) {
            for cls in obj_cls.iter_mro() {
                if let Some(f) = cls.slots.as_sequence.as_ref() {
                    return f(obj, vm).map(|x| PySequenceMethodsRef(x));
                }
            }
        }
        Err(vm.new_type_error(format!(
            // TODO(snowapril) : fix type error message like CPython spec
            "a bytes-like object is required, not '{}'",
            obj_cls.name
        )))
    }
}

impl Deref for PySequenceMethodsRef {
    type Target = dyn PySequenceMethods;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl PySequenceMethodsRef {
    pub fn new(seq: impl PySequenceMethods + 'static) -> Self {
        Self(Box::new(seq))
    }
    pub fn into_rcbuf(self) -> RcSequenceMethods {
        let this = std::mem::ManuallyDrop::new(self);
        let seq_box = unsafe { std::ptr::read(&this.0) };
        RcSequenceMethods(seq_box.into())
    }
}

impl From<Box<dyn PySequenceMethods>> for PySequenceMethodsRef {
    fn from(seq: Box<dyn PySequenceMethods>) -> Self {
        PySequenceMethodsRef(seq)
    }
}

#[derive(Debug, Clone)]
pub struct RcSequenceMethods(PyRc<dyn PySequenceMethods>);
impl Deref for RcSequenceMethods {
    type Target = dyn PySequenceMethods;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl PySequenceMethods for RcSequenceMethods {
    fn length(&self) -> PyResult<usize> {
        self.0.length()
    }
    
    fn concat(&self, other: PyObjectRef, vm: &VirtualMachine) -> PyResult {
        self.0.concat(other, vm)
    }
    
    fn repeat(&self, count: usize, vm: &VirtualMachine) -> PyResult {
        self.0.repeat(count, vm)
    }
    
    fn concat_inplace(&self, other: PyObjectRef, vm: &VirtualMachine) -> PyObjectRef {
        self.0.concat_inplace(other, vm)
    }
    
    fn repeat_inplace(&self, other: PyObjectRef, vm: &VirtualMachine) -> PyObjectRef {
        self.0.repeat_inplace(other, vm)
    }
    
    fn get_slice(&self, i1: isize, i2: isize, vm: &VirtualMachine) -> PyResult {
        self.0.get_slice(i1, i2, vm)
    }
    
    fn set_slice(&self, i1: isize, i2: isize, v: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
        self.0.set_slice(i1, i2, v, vm)
    }

    fn del_slice(&self, i1: isize, i2: isize, vm: &VirtualMachine) -> PyResult<()> {
        self.0.del_slice(i1, i2, vm)
    }
    
    fn get_item(&self, i: isize, vm: &VirtualMachine) -> PyResult {
        self.0.get_item(i, vm)
    }

    fn set_item(&self, i: isize, v: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
        self.0.set_item(i, v, vm)
    }

    fn del_item(&self, i: isize, vm: &VirtualMachine) -> PyResult<()> {
        self.0.del_item(i, vm)
    }

    fn count(&self, value: PyObjectRef, vm: &VirtualMachine) -> PyResult<usize> {
        self.0.count(value, vm)
    }
    
    fn contains(&self, value: PyObjectRef, vm: &VirtualMachine) -> PyResult<bool> {
        self.0.contains(value, vm)
    }

    fn index(&self, value: PyObjectRef, vm: &VirtualMachine) -> PyResult<usize> {
        self.0.index(value, vm)
    }
    
    fn to_vec(&self, vm: &VirtualMachine) -> PyResult<Vec<PyObjectRef>> {
        self.0.to_vec(vm)
    }
}