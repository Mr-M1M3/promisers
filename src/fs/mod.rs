use crate::{keeper::Keeper, promise::Promise};
pub use std::io::Error as IoError;
use std::{
    cell::RefCell,
    path::Path,
    rc::Rc,
    sync::{Arc, Mutex},
};
pub fn read_to_string<P: AsRef<Path>>(
    path: P,
    keeper: Rc<RefCell<Keeper>>,
) -> Rc<RefCell<Promise<String, IoError>>> {
    let path = path.as_ref().to_owned();
    return Promise::new(
        |acc, rej, passed_keeper| {
            let mut keeper = passed_keeper.borrow_mut();
            let file_operation_result = Arc::new(Mutex::new(Ok(String::new())));
            let file_operation_result_mask = file_operation_result.clone();
            let waker = keeper
                .reg_waker(|| {
                    match Arc::into_inner(file_operation_result_mask)
                        .unwrap()
                        .into_inner()
                        .unwrap()
                    {
                        Ok(content) => acc(content),
                        Err(error) => rej(error),
                    }
                })
                .unwrap();
            keeper
                .threadpool()
                .run(move || {
                    // let file_operation_result = ;
                    match std::fs::read_to_string(path) {
                        Ok(content) => {
                            *file_operation_result.lock().unwrap() = Ok(content);
                            waker.wake().unwrap();
                        }
                        Err(e) => {
                            *file_operation_result.lock().unwrap() = Err(e);
                            waker.wake().unwrap();
                        }
                    }
                })
                .unwrap();
        },
        keeper,
    );
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::keeper::Keeper;

    use super::read_to_string;
    #[test]
    fn read_file() {
        let keeper = Rc::new(RefCell::new(Keeper::with_threads(4)));
        read_to_string("./dump/promise.js", keeper.clone())
            .borrow_mut()
            .then(|content| {
                println!("read");
                println!("{content}");
            })
            .catch(|err| {
                println!("error");
                println!("{err}")
            });

            keeper.borrow_mut().keep();
    }
}
