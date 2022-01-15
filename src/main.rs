use std::cmp::Ordering::{Equal, Greater, Less};
use std::collections::HashMap;
use std::fs::File;
use std::fs::{self, metadata};
use std::io::{self, Read};
use std::io::{Seek, Write};
use std::path::{Path, PathBuf};
use wkhtmltopdf::PdfApplication;
use zip::read::ZipFile;
use zip::result::ZipResult;
use zip::ZipArchive;

// Скачивание файла zip:
async fn get_zip(need_url: &String) -> anyhow::Result<String> {
    let download_zip = reqwest::get(need_url).await?.bytes().await?;

    let file_name: String = "./download.zip".to_string();
    let path: &Path = Path::new(&file_name);
    let mut file: File = match File::create(&path) {
        Ok(file) => file,
        Err(err) => panic!("Error create file --> {}", err),
    };

    file.write_all(&download_zip)?;

    Ok(file_name)
}

fn browse_zip_archive<T, F, U>(buf: &mut T, browse_func: F) -> ZipResult<Vec<U>>
where
    T: Read + Seek,
    F: Fn(&ZipFile) -> ZipResult<U>,
{
    let mut archive: ZipArchive<&mut T> = ZipArchive::new(buf)?;
    (0..archive.len())
        .map(|i| archive.by_index(i).and_then(|file| browse_func(&file)))
        .collect()
}

// извлечение из zip архива
async fn extract_from_zip() -> String {
    let fname: &Path = std::path::Path::new("./download.zip");
    let f: File = match File::open(&fname) {
        Ok(res) => res,
        Err(err) => panic!("Error 1 {}", err),
    };

    let mut archive: ZipArchive<&File> = zip::ZipArchive::new(&f).unwrap();

    for i in 0..archive.len() {
        let mut file: ZipFile = archive.by_index(i).unwrap();

        let outpath: PathBuf = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        {
            let comment: &str = file.comment();
            if !comment.is_empty() {
                println!("File {} comment: {}", i, comment);
            }
        }

        if (&*file.name()).ends_with('/') {
            fs::create_dir_all(&outpath).unwrap();
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p).unwrap();
                }
            }
            let mut outfile: File = fs::File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }
    }

    let file: ZipFile = archive.by_index(0).unwrap();

    let outpath: PathBuf = match file.enclosed_name() {
        Some(path) => path.to_owned(),
        None => panic!("name folder is error result"),
    };

    let path_name: String = outpath.into_os_string().into_string().unwrap();

    fs::remove_file("./download.zip").unwrap();
    path_name
}

// конвертировать файл в pdf:
async fn file_to_pdf(name_folder: String) {
    let pdf_app: PdfApplication = PdfApplication::new().expect("Failed to init PDF application");
    let mut pdfout: wkhtmltopdf::PdfOutput = pdf_app
        .builder()
        .build_from_path(format!("{}/index.html", name_folder))
        .expect("failed to build pdf");

    pdfout.save("result.pdf").expect("failed to save foo.pdf");
    fs::remove_dir_all(format!("{}", name_folder)).unwrap();
}

// Проверка файла на размер и т.д:
async fn check_file_zip(need_path_download: &String) {
    // проверка зип архива
    let res_get_zip: String = match get_zip(&need_path_download).await {
        Ok(res) => res,
        Err(err) => panic!("Error {}", err),
    };

    let need_size: u64 = 2147483648;
    if let Ok(meta) = metadata("src/download.zip") {
        let _size = match meta.len().cmp(&need_size) {
            Less => {
                println!(
                    "Folder name {}, size is {} is normal",
                    &res_get_zip[2..].trim(),
                    meta.len()
                );

                // ------------------------ Проверка на нужные папки
                let mut file_info: File =
                    File::open(&res_get_zip[2..].trim()).expect("Couldn't open file");

                let folder_info: Vec<String> = match browse_zip_archive(&mut file_info, |f| {
                    Ok(format!("File name --> {}", f.name(),))
                }) {
                    Ok(files) => files,
                    Err(err) => panic!("Error info files {}", err),
                };

                let father_folder: &str = &folder_info[0][14..];
                let need_folders: Vec<String> = vec![
                    format!("{}img/", father_folder),
                    format!("{}css/", father_folder),
                    format!("{}index.html", father_folder),
                ];
                let mut list_hash: HashMap<String, bool> = HashMap::new();
                list_hash.insert(String::from("img"), false);
                list_hash.insert(String::from("css"), false);
                list_hash.insert(String::from("index.html"), false);

                for i in need_folders.iter() {
                    folder_info.iter().any(|e| {
                        if i == &e[14..].to_string() {
                            list_hash.remove(&format!(
                                "{}",
                                &i[father_folder.len()..].trim_end_matches("/")
                            ));
                            list_hash.insert(
                                format!("{}", &i[father_folder.len()..].trim_end_matches("/")),
                                true,
                            );
                            true
                        } else {
                            false
                        }
                    });
                }

                let mut count: i32 = 0;
                for (_key, value) in list_hash {
                    if value == true {
                        count += 1;
                    }
                }

                match count {
                    3 => {
                        println!("The folders that should be in the archive are the same");
                        // Вызываем ф-цию конвертации в pdf
                    }
                    _ => println!("The folders that should be in the archive do not match"),
                }
            }
            Greater => println!(
                "Folder name {}, size  {} more 2 gigabytes memory",
                &res_get_zip[2..].trim(),
                meta.len()
            ),
            Equal => println!(
                "Folder name {}, size {} equal 2 gigabytes memory",
                &res_get_zip[2..].trim(),
                meta.len()
            ),
        };
    }
}
async fn start(need_path_download: String) {
    check_file_zip(&need_path_download).await;
    let res_extract_from_zip: String = extract_from_zip().await;
    file_to_pdf(res_extract_from_zip).await;
}

#[tokio::main]
async fn main() {
    start(
        "https://github.com/Vladosya/verstka-bem-adaptive-no-mobile/archive/refs/heads/main.zip"
            .to_string(),
    )
    .await;
}
