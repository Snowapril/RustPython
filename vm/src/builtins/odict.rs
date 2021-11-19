use super::{set::PySetInner, IterStatus, PyTypeRef};
use crate::{
    builtins::{PositionIterInternal, PyDict, PyDictRef, PySet, PyTuple},
    dictdatatype::{self, DictKey},
    function::{ArgIterable, FuncArgs, KwArgs, OptionalArg},
    protocol::{PyIterIter, PyIterReturn},
    types::{
        Comparable, Constructor, IterNext, IterNextIterable, Iterable, PyComparisonOp,
        Unconstructible,
    },
    vm::{ReprGuard, VirtualMachine},
    PyArithmeticValue::NotImplemented,
    PyClassDef, PyClassImpl, PyComparisonValue, PyContext, PyObject, PyObjectRef, PyObjectView,
    PyRef, PyResult, PyValue,
};
use rustpython_common::lock::PyMutex;
use std::fmt;

/// Dictionary that remembers insertion order
#[pyclass(module = false, name = "OrderedDict", base = "PyDict")]
pub struct PyOrderedDict {
    dict: PyDictRef,
}

impl PyValue for PyOrderedDict {
    fn class(vm: &VirtualMachine) -> &PyTypeRef {
        &vm.ctx.types.odict_type
    }
}

impl fmt::Debug for PyOrderedDict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OrderedDict")
    }
}

pub type PyOrderedDictRef = PyRef<PyOrderedDict>;

#[pyimpl(flags(BASETYPE))]
impl PyOrderedDict {
    #[pymethod(magic)]
    fn init(
        &self,
        dict_obj: OptionalArg<PyObjectRef>,
        kwargs: KwArgs,
        vm: &VirtualMachine,
    ) -> PyResult<()> {
        self.update(dict_obj, kwargs, vm)
    }

    ///
    #[pymethod]
    fn update(
        &self,
        dict_obj: OptionalArg<PyObjectRef>,
        kwargs: KwArgs,
        vm: &VirtualMachine,
    ) -> PyResult<()> {
        Ok(())
    }

    ///
    #[pymethod]
    fn fromkeys(
        class: PyTypeRef,
        iterable: ArgIterable,
        value: OptionalArg<PyObjectRef>,
        vm: &VirtualMachine,
    ) -> PyResult<PyRef<Self>> {
        PyOrderedDict {
            dict: PyDict::fromkeys(class.clone(), iterable, value, vm)?,
        }
        .into_ref_with_type(vm, class)
    }
}

impl Iterable for PyOrderedDict {
    fn iter(zelf: PyRef<Self>, vm: &VirtualMachine) -> PyResult {
        Ok(PyOrderedDictKeyIterator::new(zelf.dict).into_object(vm))
    }
}

#[pyimpl]
trait ODictView: PyValue + PyClassDef + Iterable
where
    Self::ReverseIter: PyValue,
{
    type ReverseIter;

    fn dict(&self) -> &PyDictRef;
    fn item(vm: &VirtualMachine, key: PyObjectRef, value: PyObjectRef) -> PyObjectRef;

    #[pymethod(magic)]
    fn len(&self) -> usize {
        self.dict().len()
    }

    #[allow(clippy::redundant_closure_call)]
    #[pymethod(magic)]
    fn repr(zelf: PyRef<Self>, vm: &VirtualMachine) -> PyResult<String> {
        let s = if let Some(_guard) = ReprGuard::enter(vm, zelf.as_object()) {
            let mut str_parts = Vec::with_capacity(zelf.len());
            for (key, value) in zelf.dict().clone() {
                let s = &Self::item(vm, key, value).repr(vm)?;
                str_parts.push(s.as_str().to_owned());
            }
            format!("{}([{}])", Self::NAME, str_parts.join(", "))
        } else {
            "{...}".to_owned()
        };
        Ok(s)
    }

    #[pymethod(magic)]
    fn reversed(&self) -> Self::ReverseIter;
}

macro_rules! odict_view {
    ( $name: ident, $iter_name: ident, $reverse_iter_name: ident,
      $class: ident, $iter_class: ident, $reverse_iter_class: ident,
      $class_name: literal, $iter_class_name: literal, $reverse_iter_class_name: literal,
      $result_fn: expr) => {
        #[pyclass(module = false, name = $class_name)]
        #[derive(Debug)]
        pub(crate) struct $name {
            pub dict: PyDictRef,
        }

        impl $name {
            pub fn new(dict: PyDictRef) -> Self {
                $name { dict }
            }
        }

        impl ODictView for $name {
            type ReverseIter = $reverse_iter_name;
            fn dict(&self) -> &PyDictRef {
                &self.dict
            }
            fn item(vm: &VirtualMachine, key: PyObjectRef, value: PyObjectRef) -> PyObjectRef {
                $result_fn(vm, key, value)
            }
            fn reversed(&self) -> Self::ReverseIter {
                $reverse_iter_name::new(self.dict.clone())
            }
        }

        impl Iterable for $name {
            fn iter(zelf: PyRef<Self>, vm: &VirtualMachine) -> PyResult {
                Ok($iter_name::new(zelf.dict.clone()).into_object(vm))
            }
        }

        impl PyValue for $name {
            fn class(vm: &VirtualMachine) -> &PyTypeRef {
                &vm.ctx.types.$class
            }
        }

        #[pyclass(module = false, name = $iter_class_name)]
        #[derive(Debug)]
        pub(crate) struct $iter_name {
            pub size: dictdatatype::DictSize,
            pub internal: PyMutex<PositionIterInternal<PyDictRef>>,
        }

        impl PyValue for $iter_name {
            fn class(vm: &VirtualMachine) -> &PyTypeRef {
                &vm.ctx.types.$iter_class
            }
        }

        #[pyimpl(with(Constructor, IterNext))]
        impl $iter_name {
            fn new(dict: PyDictRef) -> Self {
                $iter_name {
                    size: dict.size(),
                    internal: PyMutex::new(PositionIterInternal::new(dict, 0)),
                }
            }

            #[pymethod(magic)]
            fn length_hint(&self) -> usize {
                self.internal.lock().length_hint(|_| self.size.entries_size)
            }
        }
        impl Unconstructible for $iter_name {}

        impl IterNextIterable for $iter_name {}
        impl IterNext for $iter_name {
            #[allow(clippy::redundant_closure_call)]
            fn next(zelf: &PyObjectView<Self>, vm: &VirtualMachine) -> PyResult<PyIterReturn> {
                let mut internal = zelf.internal.lock();
                let next = if let IterStatus::Active(dict) = &internal.status {
                    if dict.entries.has_changed_size(&zelf.size) {
                        internal.status = IterStatus::Exhausted;
                        return Err(vm.new_runtime_error(
                            "dictionary changed size during iteration".to_owned(),
                        ));
                    }
                    match dict.entries.next_entry(internal.position) {
                        Some((position, key, value)) => {
                            internal.position = position;
                            PyIterReturn::Return(($result_fn)(vm, key, value))
                        }
                        None => {
                            internal.status = IterStatus::Exhausted;
                            PyIterReturn::StopIteration(None)
                        }
                    }
                } else {
                    PyIterReturn::StopIteration(None)
                };
                Ok(next)
            }
        }

        #[pyclass(module = false, name = $reverse_iter_class_name)]
        #[derive(Debug)]
        pub(crate) struct $reverse_iter_name {
            pub size: dictdatatype::DictSize,
            internal: PyMutex<PositionIterInternal<PyDictRef>>,
        }

        impl PyValue for $reverse_iter_name {
            fn class(vm: &VirtualMachine) -> &PyTypeRef {
                &vm.ctx.types.$reverse_iter_class
            }
        }

        #[pyimpl(with(Constructor, IterNext))]
        impl $reverse_iter_name {
            fn new(dict: PyDictRef) -> Self {
                let size = dict.size();
                let position = size.entries_size.saturating_sub(1);
                $reverse_iter_name {
                    size,
                    internal: PyMutex::new(PositionIterInternal::new(dict, position)),
                }
            }

            #[pymethod(magic)]
            fn length_hint(&self) -> usize {
                self.internal
                    .lock()
                    .rev_length_hint(|_| self.size.entries_size)
            }
        }
        impl Unconstructible for $reverse_iter_name {}

        impl IterNextIterable for $reverse_iter_name {}
        impl IterNext for $reverse_iter_name {
            #[allow(clippy::redundant_closure_call)]
            fn next(zelf: &PyObjectView<Self>, vm: &VirtualMachine) -> PyResult<PyIterReturn> {
                let mut internal = zelf.internal.lock();
                let next = if let IterStatus::Active(dict) = &internal.status {
                    if dict.entries.has_changed_size(&zelf.size) {
                        internal.status = IterStatus::Exhausted;
                        return Err(vm.new_runtime_error(
                            "dictionary changed size during iteration".to_owned(),
                        ));
                    }
                    match dict.entries.prev_entry(internal.position) {
                        Some((position, key, value)) => {
                            if internal.position == position {
                                internal.status = IterStatus::Exhausted;
                            } else {
                                internal.position = position;
                            }
                            PyIterReturn::Return(($result_fn)(vm, key, value))
                        }
                        None => {
                            internal.status = IterStatus::Exhausted;
                            PyIterReturn::StopIteration(None)
                        }
                    }
                } else {
                    PyIterReturn::StopIteration(None)
                };
                Ok(next)
            }
        }
    };
}

odict_view! {
    PyOrderedDictKeys,
    PyOrderedDictKeyIterator,
    PyOrderedDictReverseKeyIterator,
    dict_keys_type,
    dict_keyiterator_type,
    dict_reversekeyiterator_type,
    "odict_keys",
    "odict_keyiterator",
    "odict_reversekeyiterator",
    |_vm: &VirtualMachine, key: PyObjectRef, _value: PyObjectRef| key
}

odict_view! {
    PyOrderedDictValues,
    PyOrderedDictValueIterator,
    PyOrderedDictReverseValueIterator,
    dict_values_type,
    dict_valueiterator_type,
    dict_reversevalueiterator_type,
    "odict_values",
    "odict_valueiterator",
    "odict_reversevalueiterator",
    |_vm: &VirtualMachine, _key: PyObjectRef, value: PyObjectRef| value
}

odict_view! {
    PyOrderedDictItems,
    PyOrderedDictItemIterator,
    PyOrderedDictReverseItemIterator,
    dict_items_type,
    dict_itemiterator_type,
    dict_reverseitemiterator_type,
    "odict_items",
    "odict_itemiterator",
    "odict_reverseitemiterator",
    |vm: &VirtualMachine, key: PyObjectRef, value: PyObjectRef|
        vm.new_tuple((key, value)).into()
}

// Set operations defined on set-like views of the dictionary.
#[pyimpl]
trait OrderedViewSetOps: ODictView {
    fn to_set(zelf: PyRef<Self>, vm: &VirtualMachine) -> PyResult<PySetInner> {
        let len = zelf.dict().len();
        let zelf: PyObjectRef = Self::iter(zelf, vm)?;
        let iter = PyIterIter::new(vm, zelf, Some(len));
        PySetInner::from_iter(iter, vm)
    }

    #[pymethod(name = "__rxor__")]
    #[pymethod(magic)]
    fn xor(zelf: PyRef<Self>, other: ArgIterable, vm: &VirtualMachine) -> PyResult<PySet> {
        let zelf = Self::to_set(zelf, vm)?;
        let inner = zelf.symmetric_difference(other, vm)?;
        Ok(PySet { inner })
    }

    #[pymethod(name = "__rand__")]
    #[pymethod(magic)]
    fn and(zelf: PyRef<Self>, other: ArgIterable, vm: &VirtualMachine) -> PyResult<PySet> {
        let zelf = Self::to_set(zelf, vm)?;
        let inner = zelf.intersection(other, vm)?;
        Ok(PySet { inner })
    }

    #[pymethod(name = "__ror__")]
    #[pymethod(magic)]
    fn or(zelf: PyRef<Self>, other: ArgIterable, vm: &VirtualMachine) -> PyResult<PySet> {
        let zelf = Self::to_set(zelf, vm)?;
        let inner = zelf.union(other, vm)?;
        Ok(PySet { inner })
    }

    #[pymethod(magic)]
    fn sub(zelf: PyRef<Self>, other: ArgIterable, vm: &VirtualMachine) -> PyResult<PySet> {
        let zelf = Self::to_set(zelf, vm)?;
        let inner = zelf.difference(other, vm)?;
        Ok(PySet { inner })
    }

    #[pymethod(magic)]
    fn rsub(zelf: PyRef<Self>, other: ArgIterable, vm: &VirtualMachine) -> PyResult<PySet> {
        let left = PySetInner::from_iter(other.iter(vm)?, vm)?;
        let right = ArgIterable::try_from_object(vm, Self::iter(zelf, vm)?)?;
        let inner = left.difference(right, vm)?;
        Ok(PySet { inner })
    }

    fn cmp(
        zelf: &PyObjectView<Self>,
        other: &PyObject,
        op: PyComparisonOp,
        vm: &VirtualMachine,
    ) -> PyResult<PyComparisonValue> {
        match_class!(match other {
            ref dictview @ Self => {
                PyDict::inner_cmp(
                    zelf.dict(),
                    dictview.dict(),
                    op,
                    !zelf.class().is(&vm.ctx.types.dict_keys_type),
                    vm,
                )
            }
            ref _set @ PySet => {
                let inner = Self::to_set(zelf.to_owned(), vm)?;
                let zelf_set = PySet { inner }.into_object(vm);
                PySet::cmp(zelf_set.downcast_ref().unwrap(), other, op, vm)
            }
            _ => {
                Ok(NotImplemented)
            }
        })
    }
}

impl OrderedViewSetOps for PyOrderedDictKeys {}
#[pyimpl(with(ODictView, Constructor, Comparable, Iterable, OrderedViewSetOps))]
impl PyOrderedDictKeys {
    #[pymethod(magic)]
    fn contains(zelf: PyRef<Self>, key: PyObjectRef, vm: &VirtualMachine) -> PyResult<bool> {
        zelf.dict().contains(key, vm)
    }
}
impl Unconstructible for PyOrderedDictKeys {}

impl Comparable for PyOrderedDictKeys {
    fn cmp(
        zelf: &PyObjectView<Self>,
        other: &PyObject,
        op: PyComparisonOp,
        vm: &VirtualMachine,
    ) -> PyResult<PyComparisonValue> {
        OrderedViewSetOps::cmp(zelf, other, op, vm)
    }
}

impl OrderedViewSetOps for PyOrderedDictItems {}
#[pyimpl(with(ODictView, Constructor, Comparable, Iterable, OrderedViewSetOps))]
impl PyOrderedDictItems {
    #[pymethod(magic)]
    fn contains(zelf: PyRef<Self>, needle: PyObjectRef, vm: &VirtualMachine) -> PyResult<bool> {
        let needle = match_class! {
            match needle {
                tuple @ PyTuple => tuple,
                _ => return Ok(false),
            }
        };
        if needle.len() != 2 {
            return Ok(false);
        }
        let key = needle.fast_getitem(0);
        if !zelf.dict().contains(key.clone(), vm)? {
            return Ok(false);
        }
        let value = needle.fast_getitem(1);
        let found = PyDict::getitem(zelf.dict().clone(), key, vm)?;
        vm.identical_or_equal(&found, &value)
    }
}
impl Unconstructible for PyOrderedDictItems {}

impl Comparable for PyOrderedDictItems {
    fn cmp(
        zelf: &PyObjectView<Self>,
        other: &PyObject,
        op: PyComparisonOp,
        vm: &VirtualMachine,
    ) -> PyResult<PyComparisonValue> {
        OrderedViewSetOps::cmp(zelf, other, op, vm)
    }
}

#[pyimpl(with(ODictView, Constructor, Iterable))]
impl PyOrderedDictValues {}
impl Unconstructible for PyOrderedDictValues {}

pub(crate) fn init(context: &PyContext) {
    PyOrderedDict::extend_class(context, &context.types.odict_type);
    PyOrderedDictKeys::extend_class(context, &context.types.odict_keys_type);
    PyOrderedDictKeyIterator::extend_class(context, &context.types.odict_keyiterator_type);
    PyOrderedDictReverseKeyIterator::extend_class(
        context,
        &context.types.odict_reversekeyiterator_type,
    );
    PyOrderedDictValues::extend_class(context, &context.types.odict_values_type);
    PyOrderedDictValueIterator::extend_class(context, &context.types.odict_valueiterator_type);
    PyOrderedDictReverseValueIterator::extend_class(
        context,
        &context.types.odict_reversevalueiterator_type,
    );
    PyOrderedDictItems::extend_class(context, &context.types.odict_items_type);
    PyOrderedDictItemIterator::extend_class(context, &context.types.odict_itemiterator_type);
    PyOrderedDictReverseItemIterator::extend_class(
        context,
        &context.types.odict_reverseitemiterator_type,
    );
}
