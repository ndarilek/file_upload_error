#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate yew;

use std::cell::RefCell;
use std::fs;
use std::fs::{File, ReadDir};
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use stdweb::unstable::TryInto;
use stdweb::web::*;
use stdweb::web::event::*;
use stdweb::web::html_element::*;
use yew::prelude::*;

struct Model {
    storage_directory: PathBuf,
}

enum Msg {
    StartUpload,
    Delete(String),
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
        let mut storage_directory = PathBuf::new();
        storage_directory.push("maps");
        let path = format!("/{}", storage_directory.to_str().unwrap());
        js!{
            FS.mkdir(@{path.clone()});
            FS.mount(IDBFS, {}, @{path.clone()});
            FS.syncfs(true, (err) => {
                if(err)
                    console.error(err);
            })
        }
        Self {
            storage_directory: storage_directory,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::StartUpload => {
                let element: InputElement = document().query_selector( "#import" ).unwrap().unwrap().try_into().unwrap();
                let files: FileList = js!(return @{element}.files;).try_into().unwrap();
                let filename = match files.len() {
                    0 => None,
                    _ => files.iter().nth(0).map(|f| f.name()),
                };
                let mut fh = self.storage_directory.clone();
                fh.push(filename.unwrap());
                let mut upload = Arc::new(RefCell::new(File::create(fh).expect("Failed to create file for upload")));
                let file = files.iter().nth(0).unwrap();
                let mut index = 0;
                let size = file.len();
                let offset = 1000000;
                let mut count = 0;
                while index < size {
                    let reader = Arc::new(FileReader::new());
                    let slice: Blob = js!(return @{&file}.slice(@{index as u32}, @{offset as u32});).try_into().unwrap();
                    reader.read_as_array_buffer(&slice).expect("Failed to read file");
                    let reader2 = reader.clone();
                    let upload2 = upload.clone();
                    reader.add_event_listener(move |_: LoadEndEvent| {
                        match reader2.result().unwrap() {
                            FileReaderResult::ArrayBuffer(v) => {
                                let bytes: Vec<u8> = v.into();
                                println!("Got {} bytes", bytes.len());
                                upload2.borrow_mut().write_all(&bytes).expect("Failed to write uploaded data");
                                upload2.borrow_mut().flush().expect("Failed to flush uploaded data");
                                println!("Count: {}", count);
                                if count % 100 == 0 || index+offset >= size {
                                    js! {
                                        FS.syncfs(false, (err) => {
                                            if(err)
                                                console.error(err);
                                        })
                                    };
                                }
                            },
                            _ => panic!("Should not happen")
                        }
                    });
                    index += offset;
                    count += 1;
                }
                true
            },
            Msg::Delete(file) => {
                let mut path = self.storage_directory.clone();
                path.push(file);
                fs::remove_file(path).expect("Failed to remove file");
                js!{
                    FS.syncfs(false, (err) => {
                        if(err)
                            console.error(err);
                    })
                };
                true
            },
        }
    }
}

impl Renderable<Model> for Model {
    fn view(&self) -> Html<Self> {
        let files = self.files();
        html! {
            <div>
                <table>
                    <thead>
                        <tr>
                            <th>{"Filename"}</th>
                            <th>{"Size"}</th>
                            <th>{"Actions"}</th>
                        </tr>
                    </thead>
                    <tbody> {
                        for files.unwrap().map(|file| {
                            let f = file.unwrap();
                            let metadata = f.metadata().unwrap();
                            html! {
                                <tr>
                                    <td>{f.file_name().to_str().unwrap()}</td>
                                    <td>{metadata.len()}</td>
                                    <td><button onclick=|_| Msg::Delete(f.file_name().into_string().unwrap()),>{"Delete"}</button></td>
                                </tr>
                            }
                        })
                    } </tbody>
                </table>
                <input id="import", type="file", onchange=|_| Msg::StartUpload,></input>
            </div>
        }
    }
}

impl Model {
    fn files(&self) -> io::Result<ReadDir> {
        self.storage_directory.read_dir()
    }
}

fn main() {
    yew::initialize();
    App::<Model>::new().mount_to_body();
    yew::run_loop();
}
