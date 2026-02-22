use darcy_runtime::mnist;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

fn write_gz(contents: &str, label: &str) -> String {
    let mut path = std::env::temp_dir();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    path.push(format!("darcy_mnist_{}_{}.edn.gz", label, now));
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(contents.as_bytes()).unwrap();
    let bytes = encoder.finish().unwrap();
    fs::write(&path, bytes).unwrap();
    path.to_string_lossy().to_string()
}

#[test]
fn loads_images_labels_format() {
    let data = "[[[1.0 2.0] [3.0 4.0]] [[1.0 0.0] [0.0 1.0]]]";
    let path = write_gz(data, "images_labels");
    let parsed = mnist::load_edn_gz(path.clone());
    fs::remove_file(path).ok();
    assert_eq!(parsed.images.len(), 2);
    assert_eq!(parsed.labels.len(), 2);
    assert_eq!(parsed.images[0], vec![1.0, 2.0]);
    assert_eq!(parsed.labels[1], vec![0.0, 1.0]);
}

#[test]
fn loads_sample_list_format() {
    let data = "[[[1.0 2.0] [1.0 0.0]] [[3.0 4.0] [0.0 1.0]] [[5.0 6.0] [0.0 1.0]]]";
    let path = write_gz(data, "samples");
    let parsed = mnist::load_edn_gz(path.clone());
    fs::remove_file(path).ok();
    assert_eq!(parsed.images.len(), 3);
    assert_eq!(parsed.labels.len(), 3);
    assert_eq!(parsed.images[1], vec![3.0, 4.0]);
    assert_eq!(parsed.labels[0], vec![1.0, 0.0]);
}
