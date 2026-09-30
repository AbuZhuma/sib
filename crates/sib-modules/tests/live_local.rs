use sib_core::{CollectContext, SudoMode};
use sib_transport::LocalTransport;

#[tokio::test]
#[ignore = "требует локальную машину с /proc; запускать вручную: cargo test -p sib-modules -- --ignored"]
async fn all_modules_collect_on_localhost() {
    let transport = LocalTransport::new(SudoMode::None, None);
    for module in sib_modules::default_registry().all() {
        let first = module.collect(&transport, &CollectContext::default()).await;
        assert!(first.is_ok(), "{}: {:?}", module.id(), first.err());
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let context = CollectContext {
            previous: first.ok(),
            host: "localhost".to_owned(),
            settings: Default::default(),
            checks: Vec::new(),
        };
        let second = module.collect(&transport, &context).await;
        assert!(
            second.is_ok(),
            "{} (второй сбор): {:?}",
            module.id(),
            second.err()
        );
        let first_len = context
            .previous
            .as_ref()
            .map(|s| s.samples.len())
            .unwrap_or(0);
        let second_len = second.map(|s| s.samples.len()).unwrap_or(0);
        assert!(
            second_len >= first_len,
            "{}: второй сбор потерял метрики",
            module.id()
        );
        if matches!(module.id().0, "cpu" | "network" | "disk") {
            assert!(
                second_len > first_len,
                "{}: нет метрик скорости",
                module.id()
            );
        }
    }
}

#[tokio::test]
#[ignore = "запускает и завершает локальный sleep; запускать вручную вместе с --ignored"]
async fn processes_terminate_action_stops_own_child() {
    use sib_core::{ActionRequest, Module};
    use sib_modules::processes::{ACTION_TERMINATE, ProcessesModule};

    let mut child = std::process::Command::new("sleep")
        .arg("300")
        .spawn()
        .expect("sleep");
    let transport = LocalTransport::new(SudoMode::None, None);
    let request = ActionRequest::new(ACTION_TERMINATE, child.id().to_string());
    let outcome = ProcessesModule.perform(&transport, &request).await;
    assert!(outcome.is_ok(), "{:?}", outcome.err());
    let status = child.wait().expect("wait");
    assert!(!status.success());
    let bad = ActionRequest::new(ACTION_TERMINATE, "1");
    assert!(ProcessesModule.perform(&transport, &bad).await.is_err());
}
