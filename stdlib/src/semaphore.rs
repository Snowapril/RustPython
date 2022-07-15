pub(crate) use _semaphore::make_module;

#[cfg(windows)]
#[pymodule]
mod _semaphore {
    use crate::vm::{function::ArgBytesLike, stdlib::os, PyResult, VirtualMachine};
    use winapi::um::winsock2::{self, SOCKET};
}

#[cfg(not(windows))]
#[pymodule]
mod _semaphore {
    use libc::sem_t;

    #[pyattr]
    #[pyclass(name = "SemLock")]
    #[derive(Debug, PyPayload)]
    struct PySemaphoreSemLock {
        handle: sem_t
    }

    #[derive(FromArgs)]
    struct SemLockNewArgs {
        #[pyarg(positional)]
        iterable: PyIter,
        #[pyarg(positional, optional)]
        n: OptionalArg<usize>,
    }

    impl Constructor for PySemaphoreSemLock {
        type Args = SemLockNewArgs;

        fn py_new(
            _cls: PyTypeRef,
            Self::Args { iterable, n }: Self::Args,
            vm: &VirtualMachine,
        ) -> PyResult {
            
        }
    }
    #[pyimpl(with(Constructor))]
    impl PySemaphoreSemLock {
        #[pyproperty]
        fn handle(&self) -> sem_t {

        }

        #[pyproperty]
        fn kind(&self) {

        }

        #[pyproperty]
        fn maxvalue(&self) {

        }

        #[pyproperty]
        fn name(&self) {
        }
    }
}
