pub(crate) use _warnings::make_module;

#[pymodule]
mod _warnings {
    use crate::{
        builtins::{PyStr, PyStrRef, PyTypeRef},
        frame::FrameRef,
        function::OptionalArg,
        PyObjectRef, PyResult, TypeProtocol, VirtualMachine,
    };

    #[derive(FromArgs)]
    struct WarnArgs {
        #[pyarg(positional)]
        message: PyObjectRef,
        #[pyarg(any, optional)]
        category: OptionalArg<PyTypeRef>,
        #[pyarg(any, optional)]
        stacklevel: OptionalArg<u32>,
    }

    #[pyfunction]
    fn warn(args: WarnArgs, vm: &VirtualMachine) -> PyResult<()> {
        let level = args.stacklevel.unwrap_or(1);
        let category = get_category(args.message, args.category, vm)?;
        eprintln!("level:{}: {}: {}", level, category.name(), args.message);
        Ok(())
    }

    fn get_category(
        message: PyObjectRef,
        category: OptionalArg<PyTypeRef>,
        vm: &VirtualMachine,
    ) -> PyResult<PyTypeRef> {
        let category = if message.is_instance(vm.ctx.exceptions.warning.as_object(), vm)? {
            message.clone_class()
        } else if let OptionalArg::Present(category) = category {
            category
        } else {
            vm.ctx.exceptions.user_warning.clone()
        };

        if !category.issubclass(vm.ctx.exceptions.warning) {
            return Err(vm.new_type_error(format!(
                "category must be a Warning subclass, not {}",
                category.name()
            )));
        }

        Ok(category)
    }

    fn do_warn(
        message: PyObjectRef,
        category: PyTypeRef,
        stacklevel: u32,
        vm: &VirtualMachine,
    ) -> PyObjectRef {
    }

    fn setup_context(stacklevel: u32) -> (PyObjectRef, u32, PyObjectRef, PyObjectRef) {
        // PyThreadState *tstate = _PyThreadState_GET();
        // PyFrameObject *f = PyThreadState_GetFrame(tstate);
        if stacklevel == 0 || is_internal_frame(f) {}
    }

    fn is_internal_frame(frame: FrameRef) -> bool {
        let code = frame.f_code();
        let filename = code.co_filename().as_str();

        if !filename.contains("importlib") {
            false
        } else {
            filename.contains("_bootstrap")
        }
    }

    fn warn_explicit(
        category: PyTypeRef,
        message: PyObjectRef,
        filename: PyObjectRef,
        lineno: u32,
        module: Option<PyObjectRef>,
        registry: PyObjectRef,
        source_line: PyObjectRef,
        source: PyObjectRef,
    ) -> PyResult<()> {
        if module.is_none() {
            return Ok(());
        }
    }
}
