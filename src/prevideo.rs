use std::sync::{atomic::AtomicU32, Arc};

struct DropGuard {
    on_drop: Box<dyn Fn() + Send + Sync>,
}
impl Drop for DropGuard {
    fn drop(&mut self) {
        (self.on_drop)();
    }
}
pub async fn transaction_important_operations() -> u32 {
    let opcount = Arc::new(AtomicU32::new(0));

    // Make sure all the operations happen
    let _guard = {
        let opcount = opcount.clone();
        DropGuard {
            on_drop: Box::new(move || {
                let count = opcount.load(std::sync::atomic::Ordering::Relaxed);
                if count != 3 {
                    panic!("Transaction dropped before completion: {count}");
                }
            }),
        }
    };

    // Important task 1
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    opcount.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Important task 2 that needs to complete or the state is inconsistent
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    opcount.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Important task 3 that needs to complete or the state is inconsistent
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    opcount.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    opcount.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transaction_important_operations() {
        let opcount = transaction_important_operations().await;
        assert_eq!(opcount, 3);
    }

    #[tokio::test]
    #[should_panic(expected = "Transaction dropped before completion")]
    async fn panics_when_incomplete() {
        let one_second_timer = tokio::time::sleep(tokio::time::Duration::from_secs_f32(1.5));
        // This will panic because the transaction is dropped before completion
        tokio::select! {
            _ = one_second_timer => {},
            _ = transaction_important_operations() => {},
        }
    }

    #[tokio::test]
    async fn does_not_panics_when_in_task() {
        let one_second_timer = tokio::time::sleep(tokio::time::Duration::from_secs_f32(1.5));
        let tasked_operation = tokio::task::spawn(transaction_important_operations());
        // This will not panic because the transaction is running in a task, so only the task handle is dropped
        tokio::select! {
            _ = one_second_timer => {},
            _ = tasked_operation => {},
        }
    }
}
