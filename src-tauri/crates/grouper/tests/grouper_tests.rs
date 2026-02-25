use std::fs;
use std::path::Path;

use image::{DynamicImage, RgbImage};

use hagitori_grouper::{cleanup_chapter, create_archive, GroupFormat};

fn create_test_image(path: &Path) {
    let mut img = RgbImage::new(100, 100);
    for pixel in img.pixels_mut() {
        *pixel = image::Rgb([128, 128, 128]);
    }
    DynamicImage::ImageRgb8(img).save(path).unwrap();
}

fn setup_chapter_dir(dir: &Path, num_pages: usize) {
    fs::create_dir_all(dir).unwrap();
    for i in 1..=num_pages {
        create_test_image(&dir.join(format!("{:04}.jpg", i)));
    }
}

#[test]
fn create_archive_from_chapter_dir() {
    let dir = tempfile::tempdir().unwrap();
    let chapter_dir = dir.path().join("Cap. 1");
    setup_chapter_dir(&chapter_dir, 3);

    let result = create_archive(&chapter_dir, None, GroupFormat::Cbz, None).unwrap();

    assert!(result.exists());
    assert_eq!(result.extension().unwrap(), "cbz");

    // verify CBZ contents
    let file = fs::File::open(&result).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    assert_eq!(archive.len(), 3);

    // verify names are sorted
    let names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_string())
        .collect();
    assert_eq!(names, vec!["0001.jpg", "0002.jpg", "0003.jpg"]);
}

#[test]
fn create_zip_format() {
    let dir = tempfile::tempdir().unwrap();
    let chapter_dir = dir.path().join("Cap. 1");
    setup_chapter_dir(&chapter_dir, 2);

    let result = create_archive(&chapter_dir, None, GroupFormat::Zip, None).unwrap();

    assert!(result.exists());
    assert_eq!(result.extension().unwrap(), "zip");
}

#[test]
fn create_archive_with_custom_output_path() {
    let dir = tempfile::tempdir().unwrap();
    let chapter_dir = dir.path().join("Cap. 1");
    setup_chapter_dir(&chapter_dir, 2);

    let output = dir.path().join("custom_output.cbz");
    let result = create_archive(&chapter_dir, Some(&output), GroupFormat::Cbz, None).unwrap();

    assert_eq!(result, output);
    assert!(output.exists());
}

#[test]
fn create_archive_fails_for_nonexistent_dir() {
    let dir = tempfile::tempdir().unwrap();
    let result = create_archive(&dir.path().join("nonexistent"), None, GroupFormat::Cbz, None);
    assert!(result.is_err());
}

#[test]
fn create_archive_fails_for_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let chapter_dir = dir.path().join("empty");
    fs::create_dir_all(&chapter_dir).unwrap();

    let result = create_archive(&chapter_dir, None, GroupFormat::Cbz, None);
    assert!(result.is_err());
}

#[test]
fn cleanup_chapter_behavior() {
    let dir = tempfile::tempdir().unwrap();
    let chapter_dir = dir.path().join("Cap. 1");
    setup_chapter_dir(&chapter_dir, 2);

    assert!(chapter_dir.exists());
    cleanup_chapter(&chapter_dir).unwrap();
    assert!(!chapter_dir.exists());

    // nonexistent dir should also be ok
    assert!(cleanup_chapter(&dir.path().join("nonexistent")).is_ok());
}

#[test]
fn cbz_ignores_non_image_files() {
    let dir = tempfile::tempdir().unwrap();
    let chapter_dir = dir.path().join("Cap. 1");
    fs::create_dir_all(&chapter_dir).unwrap();

    create_test_image(&chapter_dir.join("0001.jpg"));
    fs::write(chapter_dir.join("metadata.json"), "{}").unwrap();
    fs::write(chapter_dir.join("readme.txt"), "test").unwrap();

    let result = create_archive(&chapter_dir, None, GroupFormat::Cbz, None).unwrap();

    let file = fs::File::open(&result).unwrap();
    let archive = zip::ZipArchive::new(file).unwrap();
    assert_eq!(archive.len(), 1); // only the image, non-image files excluded
}
