use sib_core::{CollectContext, ModuleId, SudoMode};
use sib_transport::LocalTransport;

#[tokio::main]
async fn main() {
    let id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "system".to_owned());
    let transport = LocalTransport::new(SudoMode::None, None);
    let registry = sib_modules::default_registry();
    let leaked: &'static str = Box::leak(id.into_boxed_str());
    let Some(module) = registry.get(ModuleId(leaked)) else {
        eprintln!("нет такого модуля");
        return;
    };
    let context = CollectContext {
        previous: None,
        host: "localhost".to_owned(),
        settings: Default::default(),
        checks: Vec::new(),
    };
    match module.collect(&transport, &context).await {
        Ok(snapshot) => println!("{:#?}", snapshot.data),
        Err(error) => eprintln!("{error}"),
    }
}
