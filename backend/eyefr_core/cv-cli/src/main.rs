#![allow(unused)] //wannings are distractio me, away with you!
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    self,
    env::args,
    fs::DirEntry,
    io,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use futures::{stream, StreamExt};
use reqwest::{multipart, Body, Client, Error};
use tokio::{fs, fs::File};
use tokio_util::codec::{BytesCodec, FramedRead};
const PARALLEL_REQUESTS: usize = 4;

#[derive(Parser)]
#[command(author="Ryan Lee Martin <ryan@eyemetric.com",version,about="basic enrollment tasks", long_about = None)]
struct Cli {
    #[arg(long, value_name = "URL")]
    url: Option<String>,
    #[arg(long, value_name = "PARALLEL REQUESTS")]
    par: Option<u8>,
    #[command(subcommand)]
    command: Option<Commands>,
}
#[derive(Subcommand)]
enum Commands {
    Reset,
    Enroll {
        #[arg(short, long, value_name = "PATH")]
        path: Option<PathBuf>,
        #[arg(long, value_name = "INDEX FILE")]
        csv: Option<String>,
    },

    //maybe only enroll from index, passed from std in
    Index {
        #[arg(short, long, value_name = "SEARCH TERM")]
        search: Option<String>,
        #[arg(long, value_name = "COMP ID")]
        comp: Option<u32>,
        #[arg(long, value_name = "CLIENT TYPE")]
        client: Option<String>,
    },

    #[clap(name = "delete-enroll", about = "delete a known enrollment")]
    DeleteEnrollment {
        #[arg(short, long, value_name = "NAME (Last, First)")]
        name: Option<String>,
        #[arg(short, long, value_name = "FR ID ")]
        id: Option<String>,
    },

    #[clap(name = "search-enroll", about = "Search for known enrollments")]
    SearchEnrollment {
        #[arg(short, long, value_name = "NAME (Last, First)")]
        name: Option<String>,
        #[arg(short, long, value_name = "FR ID ")]
        id: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let url = cli.url.unwrap_or("http://localhost:3000".to_string());

    match &cli.command {
        None => {
            eprintln!("you must provide a command!");
            return Ok(());
        }
        Some(Commands::Reset) => {
            println!("Oh boy, I hope you don't get fired! Everythang is gone");
            //let addr = "http://192.168.0.204:3033";
            let api = format!("{}/fr/v2/enrollment/reset", url);
            reset_enrollments(api).await?;
        }
        Some(Commands::Enroll { path, csv }) => {
            if path.is_some() && csv.is_some() {
                eprintln!("you must choose path or csv, not both");
                return Ok(());
            }
            if path.is_none() && csv.is_none() {
                eprintln!("you must provide an image path OR a csv file");
                return Ok(());
            }

            if let Some(pth) = path {
                println!("we want to enroll people from disk and we have images");
                //let addr = "http://192.168.0.204:3033";
                let api = format!("{}/fr/v2/enrollment/create", url);

                let now = Instant::now();
                let r = enroll_from_dir(api, pth).await?;
                let elapsed = now.elapsed();
                println!("enrollment took : {:.2?}!", elapsed)
            }

            if let Some(csv) = csv {
                println!("Enrolling people from an index made from search results");
                let api = format!("{}/fr/v2/enrollment/create", url);
                println!("enroll_url: {:?}", &api);
                let now = Instant::now();
                let res = enroll_from_idx(api, csv).await?;
                let elapsed = now.elapsed();
                println!("enrollment took : {:.2?}!", elapsed)
            }

            println!("We have enrolled the things!");
        }
        Some(Commands::Index { search, comp, client }) => {
            println!("Oh you want a fancy shmancy index file?! Sure!");

            if let Some(srch) = search {
                let Some(comp) = comp else {
                    eprintln!("You must narrow your search to a single company.");
                    return Ok(());
                };

                let client = client.clone().unwrap_or("Student".to_string());
                let comp = *comp;
                //let mut output_file = "".to_string();
                let mut output_file = format!("{}-{}-{}.csv", &comp, &client, &srch);
                let mut sterm = srch.clone();

                if srch.contains("..=") {
                    println!("looking for that range, son");
                    println!("{}", &srch);

                    //TPASS likes uppercase
                    let range_chars: Vec<char> = srch
                        .chars()
                        .filter(|c| c.is_ascii_alphabetic())
                        .map(|c| c.to_ascii_uppercase())
                        .collect();
                    let start = range_chars.first().copied().unwrap_or('A') as u8;
                    let end = range_chars.last().copied().unwrap_or('Z') as u8;
                    let range = start..=end;
                    sterm =
                        range.map(|c| (c as char).to_string()).collect::<Vec<String>>().join(",");
                    let r1 = (start as char).to_string();
                    let r2 = (end as char).to_string();
                    output_file = format!("{}-{}-{}-{}.csv", comp, client, r1, r2);
                }

                let s_req = json!({
                    "search_term": sterm,
                    "comp_id": comp,
                    "client_type": client,
                    "depth": 1,

                });

                let res = search_tpass(url, &s_req).await?;
                index_search_results(res, &output_file).await?;
            }
        }
        Some(Commands::DeleteEnrollment { name, id }) => {
            println!("Feature Coming soon");
            if let Some(name) = name {
                let s_req = json!({ "last_name": name });

                let res = search_enrollments(&url, &s_req).await?;
                format_results(&res);

                //build the delete request.
                let mut ids: Vec<&str> = vec![];

                if let Some(items) = res.as_array() {
                    for item in items {
                        if let Some(fr_id) = item["fr_id"].as_str() {
                            ids.push(fr_id);
                        }
                    }
                }

                let res = delete_enrollments(&url, &ids).await?;
                println!("{:?}", res);

                //println!("{:?}", res);
                //get the fr_ids of the name result
                //build delete request with fr_ids array
            }

            if let Some(id) = id.as_deref() {
                let res = delete_enrollments(&url, &[id]).await?;
                println!("{:?}", res);
                //we have one or more ids diret, do the delete without search.
            }
        }

        Some(Commands::SearchEnrollment { name, id }) => {
            if let Some(name) = name {
                let s_req = json!({ "last_name": name });

                let res = search_enrollments(&url, &s_req).await?;

                format_results(&res);
                //get the fr_ids of the name result
                //build delete request with fr_ids array
            }

            if id.is_some() {
                println!("Feature coming soon!");
                //we have one or more ids diret, do the delete without search.
            }
        }
    }

    println!("----------------------------------------------------------");
    Ok(())
}

fn format_results(val: &Value) {
    if let Some(items) = val.as_array() {
        for item in items {
            let fr_id = &item["fr_id"];
            let ext_id = read_ext_id(item).unwrap_or_else(|| "".to_string());
            let last_name = &item["details"]["last_name"];
            let first_name = &item["details"]["first_name"];
            let fmt = format!("{}   {}   {}, {}", fr_id, ext_id, last_name, first_name);
            println!("{}", fmt);
        }
    }
}

use csv::Writer;
use std::fs::OpenOptions;

async fn index_search_results(data: Value, output_file: &str) -> Result<()> {
    // Parse the JSON string into a Value object

    let idx_path = std::env::current_dir()?.join("index");

    match fs::create_dir_all(&idx_path).await {
        Ok(_) => println!("Successfully created the index directory"),
        Err(e) => {
            // Ignore the error if the directory already exists
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                println!("Could not create the index directory {}", e);
            } else {
                println!("Index dir exists..cool");
            }
        }
    }

    let idx_path = idx_path.join(output_file);

    println!("IDX FILE: {:?}", &idx_path);

    let include_header = true;
    // Open the output CSV file in append mode or create it if it doesn't exist
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        //.append(true)
        .open(&idx_path)
        .with_context(|| format!("failed to open index output file {}", idx_path.display()))?;

    let mut writer = Writer::from_writer(file);

    // Write the CSV header if required
    if include_header {
        writer.write_record(["last_name", "first_name", "ext_id", "url"])?;
    }

    // Check if the Value is an array
    if let Some(outer) = data.as_array() {
        for obj in outer {
            if let Some(items) = obj.as_array() {
                for item in items {
                    let ccode = item["ccode"].as_u64().map(|num| num.to_string());
                    let last_name = item["lName"].as_str().unwrap_or("");
                    let first_name = item["fName"].as_str().unwrap_or("");
                    let url = item["imgUrl"].as_str().unwrap_or("");

                    if let Some(ccode) = ccode.filter(|_| !url.is_empty()) {
                        //no image, no point
                        writer.write_record([last_name, first_name, &ccode, url]);
                    }
                }
            } else {
                println!("exptected an array. didn't get one. ignoring..");
            }
        }
    } else {
        println!("The provided JSON is not an array.");
    }

    // Flush the CSV writer to ensure everything is written to the file
    writer.flush()?;

    Ok(())
}

async fn delete_enrollments(url: &str, ids: &[&str]) -> Result<Vec<Value>> {
    let client = reqwest::Client::new();
    let api = format!("{}/fr/v2/enrollment/delete", url);
    println!("{:?}", &api);
    let mut vals: Vec<Value> = vec![];
    //one at a time, remember we want parr
    for id in ids {
        let del_req = json!({ "fr_id": id });

        println!("{:?}", del_req);

        let response = client.post(&api).json(&del_req).send().await?;
        let rj = response.json().await?;
        vals.push(rj);
    }
    Ok(vals)
}

async fn search_enrollments(url: &str, search_req: &Value) -> Result<Value> {
    let client = reqwest::Client::new();
    let api = format!("{}/fr/v2/enrollment/search", url);
    println!("{:?}", &api);
    let response = client.post(api).json(search_req).send().await?;
    let rj = response.json().await?;
    //println!("{:?}", &rj);
    Ok(rj)
}

async fn search_tpass(url: String, search_req: &Value) -> Result<Value> {
    let client = reqwest::Client::new();
    let api = format!("{}/tpass/search", url);
    println!("{:?}", &api);
    let response = client.post(api).json(search_req).send().await?;
    let rj = response.json().await?;
    //println!("{:?}", &rj);
    Ok(rj)
}

async fn reset_enrollments(url: String) -> Result<Value> {
    let client = reqwest::Client::new();
    println!("{:?}", &url);
    let response = client.post(url).body("").send().await?;
    let rj = response.json().await?;
    println!("{:?}", &rj);
    Ok(rj)
}

// writer.write_record(&["last_name", "first_name","ext_id", "url" ])?;
#[derive(Debug, Deserialize)]
struct Record {
    last_name: String,
    first_name: String,
    ext_id: String,
    url: String,
}

fn read_ext_id(item: &Value) -> Option<String> {
    item.get("ext_id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| item.get("ext_id").and_then(Value::as_u64).map(|num| num.to_string()))
}

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

async fn enroll_from_idx(url: String, pth: &str) -> Result<Value> {
    let m_prog = Arc::new(MultiProgress::new());
    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?
    .progress_chars("##-");

    let def_style = ProgressStyle::default_spinner();

    let idx_file = std::env::current_dir()?.join("index").join(pth);
    if !idx_file.exists() {
        println!("index file doesn't exist. You spell it right, dummy?");
        return Ok(json!({}));
    }
    println!("csv: {:?}", &idx_file);

    let client = reqwest::Client::new();

    let contents = std::fs::read_to_string(&idx_file)
        .with_context(|| format!("failed reading csv index file {}", idx_file.display()))?;
    let mut rdr = csv::ReaderBuilder::new().from_reader(contents.as_bytes());

    let recs: std::result::Result<Vec<Record>, _> = rdr.deserialize().collect(); //.collect();
    let recs: Vec<Record> = recs?;

    //let prog = Arc::new(ProgressBar::new_spinner());
    let prog = Arc::new(ProgressBar::new(recs.len() as u64));
    //let prog = Arc::new(ProgressBar::new(179));

    //let futures = stream::iter(rdr.deserialize::<Record>()).map(|item| {
    let futures = stream::iter(recs)
        .map(|item| {
            //prog.set_style(def_style.clone());
            prog.set_style(sty.clone());

            let client = client.clone();
            let url = url.clone();
            let item: Record = item;

            let lname = item.last_name.clone();
            let details = json!({
                "kind": "Min",
                "ext_id": item.ext_id,
                "last_name": item.last_name,
                "first_name": item.first_name
            });

            let mp: Arc<ProgressBar> = Arc::clone(&prog);

            async move {
                mp.inc(1);
                let r = enroll(url, &details, None, &client).await?;
                mp.set_message(format!("{},  {}", lname, item.first_name));
                Ok(r)
            }
        })
        .buffer_unordered(PARALLEL_REQUESTS);

    let e_res = futures
        .fold(Vec::new(), |mut acc, res: Result<Value>| async move {
            match res {
                Ok(data) => {
                    acc.push(data);
                    acc
                }
                Err(e) => {
                    println!("There was an error with the tpass call: {}", e); //a great place to log
                    acc
                }
            }
        })
        .await;

    Ok(json!({}))
}

async fn enroll_from_dir(url: String, pth: &PathBuf) -> Result<Value> {
    let client = reqwest::Client::new();

    let escape_count = 100;
    let mut counter = 0;

    let files = std::fs::read_dir(pth)?;
    let mut paths = vec![];
    for file in files {
        match file {
            Ok(entry) => paths.push(entry.path()),
            Err(e) => eprintln!("couldn't read directory entry: {}", e),
        }
    }

    let futures = stream::iter(paths)
        .map(|pth| {
            let client = client.clone();
            let url = url.clone();

            async move {
                let Some(ff) = pth.file_stem().and_then(|s| s.to_str()) else {
                    println!("Filename doesn't match expected UTF-8 pattern");
                    return Ok::<Value, anyhow::Error>(json!({
                        "msg": "filename doesn't match pattern"
                    }));
                };

                let fparts: Vec<&str> = ff.split('_').collect();
                if fparts.len() < 3 {
                    println!("Filename doesn't match pattern ID_Last_First.[jpeg|jpg|png]");
                    return Ok(json!({ "msg": "filename doesn't match pattern" }));
                }

                let details = json!({
                    "kind": "Min",
                    "ext_id": fparts[0],
                    "last_name": fparts[1],
                    "first_name": fparts[2]
                });

                println!("{:?}", pth.to_str());
                let file = File::open(pth).await?;
                let r = enroll(url, &details, Some(file), &client).await?;
                Ok(r)
            }
        })
        .buffer_unordered(PARALLEL_REQUESTS);

    let _e_res = futures
        .fold(Vec::new(), |mut acc, res: Result<Value>| async move {
            match res {
                Ok(data) => {
                    acc.push(data);
                }
                Err(e) => {
                    eprintln!("There was an error during enrollment: {}", e);
                }
            }
            acc
        })
        .await;

    Ok(json!({}))
}

//async fn enroll(url: String, pth: PathBuf, client: &Client) -> Result<Value, reqwest::Error> {
async fn enroll(
    url: String,
    details: &Value,
    file: Option<File>,
    client: &Client,
) -> Result<Value> {
    let val = json!({});
    let det_str =
        serde_json::to_string(details).context("failed to serialize enrollment details")?;

    let form = match file {
        Some(f) => reqwest::multipart::Form::new()
            .part("image", reqwest::multipart::Part::stream(f))
            .text("details", det_str.clone()),
        None => reqwest::multipart::Form::new()
            .part("image", reqwest::multipart::Part::stream(""))
            .text("details", det_str),
    };

    let response = client.post(url).multipart(form).send().await?;
    //println!("Enroll Response: {:?}", response);

    Ok(json!({}))
}

//what were we going to use this for?
async fn post_b64_body(
    url: String,
    b64: String,
    opt: (&str, String),
    client: &Client,
) -> Result<Value> {
    let image_data = &[
        ("image", format!("data:image/jpeg;name=file.jpeg;base64,{}", b64)), //TODO: use opts to determine filetype
        opt,
    ];

    let res: Value = client.post(url).form(image_data).send().await?.json().await?;
    Ok(res)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ImageData {
    image: Option<String>,
    opts: Option<Opts>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Opts {
    filetype: String,
}

//old stuff

// for file in  fs::read_dir(&cli.path)? {

//     let url = url.clone();
//     let f = file.unwrap();
//     let pth = f.path();

//     let res = enroll(url, pth, &client).await;

//     match res {
//         Ok(r) => {
//             println!("{:?}", r);
//         },
//         Err(e) => {
//             println!("{:?}", e);
//         }
//     }

//     // counter = counter +1;
//     // if counter == escape_count {
//     //     break;
//     // }
// }
