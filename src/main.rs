use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use prettytable::{Table, Row, Cell, row, cell, format::{FormatBuilder, LinePosition, LineSeparator}};
use humansize::{FileSize, file_size_opts::CONVENTIONAL};

use sarc::{SarcFile, Endian, SarcEntry};
use zip::{CompressionMethod, ZipArchive, ZipWriter, result::ZipError, write::FileOptions};

use structopt::StructOpt;

#[derive(StructOpt, Debug, Clone)]
struct Args {
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt, Debug, Clone)]
enum Command {
    #[structopt(alias = "z")]
    Zip {
        #[structopt(short, long, alias = "compress", alias = "c")]
        yaz0: bool,
        #[structopt(short, long, conflicts_with = "yaz0")]
        zstd: bool,

        #[structopt(short, long, alias = "big")]
        big_endian: bool,
        #[structopt(short, long, alias = "little", conflicts_with = "big")]
        little_endian: bool,

        in_dir: PathBuf,
        out_file: PathBuf,
    },
    #[structopt(alias = "u", alias = "x", alias = "extract")]
    Unzip {
        in_file: PathBuf,
        out_dir: Option<PathBuf>,
    },
    IntoZip {
        in_file: PathBuf,
        out_file: PathBuf,
    },
    FromZip {
        #[structopt(short, long, alias = "compress", alias = "c")]
        yaz0: bool,
        #[structopt(short, long, conflicts_with = "yaz0")]
        zstd: bool,

        #[structopt(short, long, alias = "big")]
        big_endian: bool,
        #[structopt(short, long, alias = "little", conflicts_with = "big")]
        little_endian: bool,

        in_file: PathBuf,
        out_file: PathBuf,
    },
    #[structopt(alias = "-l", alias = "l")]
    List {
        #[structopt(short, long)]
        byte_count: bool,
        in_file: PathBuf,
    }
}

fn size(size: usize, byte_count: bool) -> String {
    if byte_count {
        size.to_string()
    } else {
        size.file_size(CONVENTIONAL).unwrap()
    }
}

fn hex(byte: &u8) -> String {
    format!("{:02X}", byte)
}

fn byte_char(byte: &u8) -> char {
    match *byte as char {
        c @ ' '..='~' => c,
        _ => '.'
    }
}

fn list(in_file: PathBuf, byte_count: bool) {
    let sarc = SarcFile::read_from_file(in_file).unwrap();
    println!("Endian: {}", match sarc.byte_order {
        Endian::Little => "Little",
        Endian::Big => "Big"
    });
    let mut table = Table::new();
    let mut total_size = 0;
    table.set_titles(row![
        c->"Size", c->"Name", c->"First bytes"
    ]);
    table.set_format(
        FormatBuilder::new()
            .column_separator(' ')
            .borders(' ')

            .separators(&[
                LinePosition::Title
            ], LineSeparator::new('-', ' ', ' ', ' '))
            .build()
    );
    for file in &sarc.files {
        let name = file.name.as_ref().map(|n| &**n).unwrap_or("[no name]");
        let bytes: String = file.data[..4].iter().map(hex).collect();
        let str_bytes: String = file.data[..4].iter().map(byte_char).collect();
        let bytes = bytes + " | " + &str_bytes;
        table.add_row(row![
            size(file.data.len(), byte_count), name, bytes
        ]);
        total_size += file.data.len();
    }
    table.add_row(row![
        "--------", "", "---------------"
    ]);
    table.add_row(row![
        size(total_size, byte_count), "", format!("{} file(s)", sarc.files.len())
    ]);
    table.printstd();
}

fn endian(big: bool) -> Endian {
    if big {
        Endian::Big
    } else {
        Endian::Little
    }
}

fn write(sarc: SarcFile, out_file: PathBuf, yaz0: bool, zstd: bool) {
    if yaz0 {
        sarc.write_yaz0(&mut fs::File::create(out_file).unwrap()).unwrap()
    } else if zstd {
        sarc.write_zstd(&mut fs::File::create(out_file).unwrap()).unwrap();
    } else {
        sarc.write_to_file(out_file).unwrap();
    }
}

fn zip(yaz0: bool, zstd: bool, in_dir: PathBuf, out_file: PathBuf, byte_order: Endian) {
    let pattern = in_dir.to_string_lossy() + "/**/*.*";
    let dir = glob::glob(&pattern).unwrap();
    let files = dir.map(|child|{
        let path = child.unwrap();
        let name = Some(path.strip_prefix(&in_dir).unwrap().to_string_lossy().replace("\\", "/").into());
        let data = fs::read(path).unwrap();

        SarcEntry {
            name,
            data
        }
    }).collect();

    let sarc = SarcFile {
        byte_order,
        files
    };
    
    write(sarc, out_file, yaz0, zstd);
}

fn unzip(in_file: PathBuf, out_dir: PathBuf) {
    let sarc = SarcFile::read_from_file(in_file).unwrap();
    let mut unk = 0;
    for file in sarc.files {
        let name = if let Some(x) = file.name {
            x
        } else {
            println!("WARN: file does not have name");
            let s = format!("unk{}.bin", unk);
            unk += 1;
            s
        };

        let mut path = out_dir.clone();
        path.extend(std::iter::once(name));

        let _ = fs::create_dir_all(path.parent().unwrap());

        fs::write(path, file.data).unwrap();
    }
}

fn main() {
    let args = Args::from_args();

    match args.command {
        Command::Zip {
            yaz0, zstd, in_dir, out_file, little_endian: _, big_endian
        } => {
            zip(yaz0, zstd, in_dir, out_file, endian(big_endian));
        }
        Command::Unzip {
            in_file, out_dir
        } => {
            let out_dir = 
                out_dir.unwrap_or_else(||{
                    let mut path = in_file.parent().unwrap().to_path_buf();
                    path.push(in_file.file_stem().unwrap());
                    path
                });
            unzip(
                in_file,
                out_dir
            );
        }
        Command::FromZip {
            yaz0, zstd, in_file, out_file, big_endian, little_endian: _
        } => {
            from_zip(yaz0, zstd, in_file, out_file, endian(big_endian));
        }
        Command::IntoZip {
            in_file, out_file
        } => {
            to_zip(in_file, out_file);
        }
        Command::List { in_file, byte_count } => list(in_file, byte_count),
    }
}

pub struct SarcConverter;

fn to_zip(in_file: PathBuf, out_file: PathBuf) {
    let sarc = SarcFile::read_from_file(in_file).unwrap();
    let mut zip = ZipWriter::new(File::create(&out_file).unwrap());

    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
    for (i, file) in sarc.files.into_iter().enumerate() {
        zip.start_file(file.name.unwrap_or_else(|| format!("{}.bin", i)), options).unwrap();
        zip.write(&file.data).unwrap();
    }
}

fn from_zip(yaz0: bool, zstd: bool, in_file: PathBuf, out_file: PathBuf, byte_order: Endian) {
    let mut zip = ZipArchive::new(File::open(in_file).unwrap()).unwrap();

    let files = (0..zip.len())
        .map(|i| {
            let file = zip.by_index(i).unwrap();
            let name = Some(file.name().to_owned());
            let data = file.bytes().collect::<Result<_, _>>().unwrap();
            SarcEntry {
                name, data
            }
        })
        .collect::<Vec<_>>();

    let sarc = SarcFile {
        byte_order, files,
    };

    write(sarc, out_file, yaz0, zstd);
}

use std::fmt;

pub struct ConvertError {
    pub message: String,
    pub kind: ConvertErrorKind,
}

impl fmt::Debug for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ConvertError '{:?}', message: \"{}\"", self.kind, self.message)
    }
}

#[derive(Debug)]
pub enum ConvertErrorKind {
    Param,
    Nus3audio,
    Msc,
    File,
    HandleNone,
    YamlError,
    Utf8Error,
    ParseIntError,
    MessageFormat,
    WaveError,
    SarcError,
    ZipError,
    Byml,
    Yaz0Error
}

impl ConvertError {
    pub fn param(message: &str) -> ConvertError {
        ConvertError {
            message: message.to_string(),
            kind: ConvertErrorKind::Param
        }
    }

    pub fn nus3audio(message: &str) -> ConvertError {
        ConvertError {
            message: message.to_string(),
            kind: ConvertErrorKind::Nus3audio
        }
    }

    pub fn file(message: &str) -> ConvertError {
        ConvertError {
            message: message.to_string(),
            kind: ConvertErrorKind::File
        }
    }

    pub fn msc(message: &str) -> ConvertError {
        ConvertError {
            message: message.to_string(),
            kind: ConvertErrorKind::Msc
        }
    }

    pub fn message_format(message: &str) -> ConvertError {
        ConvertError {
            message: message.to_string(),
            kind: ConvertErrorKind::MessageFormat
        }
    }

    pub fn byml<S: AsRef<str>>(message: S) -> ConvertError {
        ConvertError {
            message: message.as_ref().to_string(),
            kind: ConvertErrorKind::Byml,
        }
    }
}

impl std::convert::From<std::io::Error> for ConvertError {
    fn from(err: std::io::Error) -> Self {
        ConvertError {
            message: format!("{:?}", err),
            kind: ConvertErrorKind::File,
        }
    }
}

impl std::convert::From<std::str::Utf8Error> for ConvertError {
    fn from(err: std::str::Utf8Error) -> Self {
        ConvertError {
            message: format!("{:?}", err),
            kind: ConvertErrorKind::Utf8Error,
        }
    }
}

impl std::convert::From<std::num::ParseIntError> for ConvertError {
    fn from(err: std::num::ParseIntError) -> Self {
        ConvertError {
            message: format!("{:?}", err),
            kind: ConvertErrorKind::ParseIntError,
        }
    }
}

impl std::convert::From<sarc::parser::Error> for ConvertError {
    fn from(err: sarc::parser::Error) -> Self {
        ConvertError {
            message: format!("SarcParseError: {:?}", err),
            kind: ConvertErrorKind::SarcError
        }
    }
}

impl std::convert::From<sarc::writer::Error> for ConvertError {
    fn from(err: sarc::writer::Error) -> Self {
        ConvertError {
            message: format!("SarcWriteError: {:?}", err),
            kind: ConvertErrorKind::SarcError
        }
    }
}

impl std::convert::From<zip::result::ZipError> for ConvertError {
    fn from(err: zip::result::ZipError) -> Self {
        ConvertError {
            message: format!("ZipError: {:?}", err),
            kind: ConvertErrorKind::ZipError
        }
    }
}
