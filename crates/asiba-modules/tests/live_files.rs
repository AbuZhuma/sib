use asiba_core::{ActionRequest, Module, QueryRequest, SudoMode};
use asiba_modules::files::{self, FilesModule};
use asiba_transport::LocalTransport;

#[tokio::test]
#[ignore = "пишет во временную папку локальной машины; запускать вручную: cargo test -p asiba-modules --test live_files -- --ignored"]
async fn list_write_read_chmod_delete_roundtrip() {
    let transport = LocalTransport::new(SudoMode::None, None);
    let module = FilesModule;
    let directory = std::env::temp_dir().join(format!("asiba-files-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("temp dir");
    let directory = directory.to_string_lossy().into_owned();
    let path = format!("{directory}/it's a \"test\".txt");
    let content = "первая строка\n$HOME %s \\ done\n";

    let write = ActionRequest::new(files::ACTION_WRITE, &path).with_argument(content);
    module.perform(&transport, &write).await.expect("write");
    let read = module
        .query(&transport, &QueryRequest::new(files::QUERY_READ, &path))
        .await
        .expect("read");
    assert_eq!(read.text, content);

    let chmod = ActionRequest::new(files::ACTION_CHMOD, &path).with_argument("600");
    module.perform(&transport, &chmod).await.expect("chmod");
    let listing = module
        .query(
            &transport,
            &QueryRequest::new(files::QUERY_LIST, &directory),
        )
        .await
        .expect("list");
    let parsed = files::parse_listing(&listing.text).expect("parse");
    let entry = parsed
        .entries
        .iter()
        .find(|e| e.name == "it's a \"test\".txt")
        .expect("entry");
    assert_eq!(entry.mode, 0o600);
    assert_eq!(entry.size as usize, content.len());

    let binary = module
        .query(
            &transport,
            &QueryRequest::new(files::QUERY_READ, "/usr/bin/ls"),
        )
        .await;
    assert!(binary.is_err());

    let nested = format!("{directory}/nested dir");
    module
        .perform(
            &transport,
            &ActionRequest::new(files::ACTION_MKDIR, &nested),
        )
        .await
        .expect("mkdir");
    let created = format!("{nested}/new.txt");
    module
        .perform(
            &transport,
            &ActionRequest::new(files::ACTION_CREATE, &created),
        )
        .await
        .expect("create");
    assert!(
        module
            .perform(
                &transport,
                &ActionRequest::new(files::ACTION_CREATE, &created)
            )
            .await
            .is_err(),
        "повторное создание должно падать"
    );
    let renamed = format!("{nested}/renamed.txt");
    module
        .perform(
            &transport,
            &ActionRequest::new(files::ACTION_MOVE, &created).with_argument(&renamed),
        )
        .await
        .expect("move");
    let copied = format!("{directory}/copy.txt");
    module
        .perform(
            &transport,
            &ActionRequest::new(files::ACTION_COPY, &renamed).with_argument(&copied),
        )
        .await
        .expect("copy");
    assert!(std::path::Path::new(&renamed).exists());
    assert!(std::path::Path::new(&copied).exists());
    assert!(!std::path::Path::new(&created).exists());

    let delete = ActionRequest::new(files::ACTION_DELETE, &directory);
    module.perform(&transport, &delete).await.expect("delete");
    assert!(!std::path::Path::new(&directory).exists());
}
