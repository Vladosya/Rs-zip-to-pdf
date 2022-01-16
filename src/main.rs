extern crate stopwatch;
use chrono::prelude::*;
use std::cmp::Ordering::{Equal, Greater, Less};
use std::collections::HashMap;
use std::fs::File;
use std::fs::{self, metadata};
use std::io::{self, Read};
use std::io::{Seek, Write};
use std::path::{Path, PathBuf};
use stopwatch::Stopwatch;
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
        Err(err) => panic!("Error create file in get_zip func ---> {}", err),
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

async fn remove_need_file(name_file: &str) {
    match Path::new(name_file).exists() {
        true => match fs::remove_file(name_file) {
            Ok(res) => res,
            Err(err) => panic!("Error remove file {} {}", name_file, err),
        },
        false => (),
    };
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
    let mut sw: Stopwatch = Stopwatch::start_new();
    let pdf_app: PdfApplication = PdfApplication::new().expect("Failed to init PDF application");
    let mut pdfout: wkhtmltopdf::PdfOutput = pdf_app
        .builder()
        .build_from_path(format!("{}/index.html", name_folder))
        .expect("failed to build pdf");

    let file_pdf_name: &str = "result.pdf";

    pdfout.save(file_pdf_name).expect("failed to save foo.pdf");
    sw.stop();
    match Path::new(file_pdf_name).exists() {
        true => {
            let mut perform_file: File = match File::create("perform_operation.txt") {
                Ok(perform_file) => perform_file,
                Err(err) => panic!("Error create file perform_operation.txt ---> {}", err),
            };
            let local: DateTime<Local> = Local::now();
            let local_date: String = match local.date().month().to_string().len() {
                1 => {
                    format!(
                        "{}:0{}:{}",
                        local.date().day(),
                        local.date().month(),
                        local.date().year()
                    )
                }
                _ => {
                    format!(
                        "{}:{}:{}",
                        local.date().day(),
                        local.date().month(),
                        local.date().year()
                    )
                }
            };
            match write!(
                &mut perform_file,
                "LOG OF EXECUTED OPERATION: \n File to convert: {}index.html \n Name file: {} \n Date of operation: date: {} time: {} \n Time spent converting: {} milliseconds",
                name_folder, file_pdf_name, local_date, &local.to_string()[11..19].trim(), sw.elapsed_ms()
            ) {
                Ok(file) => file,
                Err(err) => panic!("Error create file {}", err),
            };
        }
        false => println!("Error"),
    }
    fs::remove_dir_all(format!("{}", name_folder)).unwrap();
}

// Проверка файла на размер и т.д:
async fn check_file_zip(need_path_download: &String) {
    // проверка зип архива
    let res_get_zip: String = match get_zip(&need_path_download).await {
        Ok(res_get_zip) => res_get_zip,
        Err(err) => panic!(
            "Function call get_zip is error in check_file_zip ---> {}",
            err
        ),
    };

    let need_size: u64 = 2147483648;
    if let Ok(meta) = metadata(&res_get_zip) {
        match meta.len().cmp(&need_size) {
            Less => {
                println!(
                    "Folder name {}, size is {} is normal",
                    &res_get_zip[2..].trim(),
                    meta.len()
                );

                // ------------------------ Проверка на нужные папки
                let mut file_info: File = match File::open(&res_get_zip[2..].trim()) {
                    Ok(file_info) => file_info,
                    Err(err) => panic!("Error couldn't open file in check_file_zip ---> {}", err),
                };

                let folder_info: Vec<String> = match browse_zip_archive(&mut file_info, |f| {
                    Ok(format!("File name --> {}", f.name(),))
                }) {
                    Ok(folder_info) => folder_info,
                    Err(err) => panic!("Error get information to folder ---> {}", err),
                };

                let father_folder: &str = &folder_info[0][14..];
                let need_folders: Vec<String> = vec![
                    format!("{}img/", father_folder),
                    format!("{}css/", father_folder),
                    format!("{}style/", father_folder),
                    format!("{}index.html", father_folder),
                ];
                let mut list_hash: HashMap<String, bool> = HashMap::new();
                list_hash.insert(String::from("img"), false);
                list_hash.insert(String::from("css"), false);
                list_hash.insert(String::from("style"), false);
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
                        for i in folder_info {
                            println!("i --> {}", i);
                            if i.contains("index.html") {
                                // Вызываем ф-ции: извлечение из архива zip и после нее конвертация в pdf
                                println!("The folders that should be in the archive are the same");
                                let res_extract_from_zip: String = extract_from_zip().await;
                                file_to_pdf(res_extract_from_zip).await;
                            } else {
                                println!("The folders that should be in the archive do not match");
                                fs::remove_file("./download.zip").unwrap();
                            }
                        }
                    }
                    4 => {
                        // Вызываем ф-ции: извлечение из архива zip и после нее конвертация в pdf
                        println!("The folders that should be in the archive are the same");
                        let res_extract_from_zip: String = extract_from_zip().await;
                        file_to_pdf(res_extract_from_zip).await;
                    }
                    _ => {
                        println!("The folders that should be in the archive do not match");
                        fs::remove_file("./download.zip").unwrap();
                    }
                }
            }
            Greater => {
                println!(
                    "Folder name {}, size  {} more 2 gigabytes memory",
                    &res_get_zip[2..].trim(),
                    meta.len()
                );
                fs::remove_file("./download.zip").unwrap();
            }
            Equal => {
                println!(
                    "Folder name {}, size {} equal 2 gigabytes memory",
                    &res_get_zip[2..].trim(),
                    meta.len()
                );
                fs::remove_file("./download.zip").unwrap();
            }
        };
    }
}

async fn start(need_path_download: String) {
    remove_need_file("perform_operation.txt").await;
    remove_need_file("result.pdf").await;
    check_file_zip(&need_path_download).await;
}

#[tokio::main]
async fn main() {
    start(
        "https://github.com/Vladosya/verstka-bem-adaptive-no-mobile/archive/refs/heads/main.zip"
            .to_string(),
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_remove_need_file() {
        let file_one: &str = "perform_operation.txt";
        let file_two: &str = "result.pdf";
        remove_need_file(file_one).await;
        remove_need_file(file_two).await;
        assert_eq!(Path::new(file_one).exists(), false);
        assert_eq!(Path::new(file_two).exists(), false);
    }

    #[tokio::test]
    async fn test_extract_from_zip() {
        let need_url: String = "https://github.com/Vladosya/verstka-bem-adaptive-no-mobile/archive/refs/heads/main.zip".to_string();
        let _res_get_zip: String = match get_zip(&need_url).await {
            Ok(res) => res,
            Err(err) => panic!("Error {}", err),
        };
        let _res_extract: String = extract_from_zip().await;
        assert_eq!(Path::new("./download.zip").exists(), false);
    }

    #[tokio::test]
    async fn test_file_to_pdf() {
        let need_url: String = "https://github.com/Vladosya/verstka-bem-adaptive-no-mobile/archive/refs/heads/main.zip".to_string();
        let _res_get_zip: String = match get_zip(&need_url).await {
            Ok(res) => res,
            Err(err) => panic!("Error {}", err),
        };
        let res_extract_from_zip: String = extract_from_zip().await;
        file_to_pdf(res_extract_from_zip).await;
        assert_eq!(Path::new("./download.zip").exists(), false);
        assert_eq!(Path::new("result.pdf").exists(), true);
    }

    #[tokio::test]
    async fn test_get_zip() {
        let need_url: String = "https://github.com/Vladosya/verstka-bem-adaptive-no-mobile/archive/refs/heads/main.zip".to_string();
        let res_get_zip: String = match get_zip(&need_url).await {
            Ok(res) => res,
            Err(err) => panic!("Error {}", err),
        };

        assert_eq!(res_get_zip, "./download.zip".to_string());
    }
}
