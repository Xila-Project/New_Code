use crate::File_system::Generics;

use std::collections::HashMap;
use std::env::{current_dir, var};
use std::fs::*;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Arc, RwLock};

pub struct File_system_type {
    Virtual_root_path: Generics::Path_type,
    Mount_points: Vec<Generics::Path_type>,
    Open_files: Arc<RwLock<HashMap<Generics::File_identifier_type, File>>>,
}

impl File_system_type {
    pub fn New() -> Self {
        File_system_type {
            Virtual_root_path: Generics::Path_type::New(),
            Mount_points: Vec::new(),
            Open_files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn Register_file(&self, File: File) -> Result<Generics::File_identifier_type, ()> {
        let mut Open_files = self.Open_files.write().unwrap();

        let mut File_identifier: Generics::File_identifier_type = 0;
        while Open_files.contains_key(&File_identifier) {
            File_identifier += 1;
        }

        if Open_files.insert(File_identifier, File).is_some() {
            // If the file identifier is already used.
            return Err(());
        }
        Ok(File_identifier)
    }

    pub fn Unregister_file(
        &self,
        File_identifier: Generics::File_identifier_type,
    ) -> Result<(), ()> {
        let mut Open_files = self.Open_files.write().unwrap();
        match Open_files.remove(&File_identifier) {
            Some(_) => Ok(()),
            None => Err(()),
        }
    }

    pub fn Get_full_path(&self, Path: &Generics::Path_type) -> Generics::Path_type {
        let mut Full_path = self.Virtual_root_path.clone();
        Full_path + Path
    }
}

impl Generics::File_system_traits for File_system_type {
    fn Initialize(&mut self) -> Result<(), ()> {
        match var("Xila_virtual_root_path") {
            Ok(value) => {
                self.Virtual_root_path = value.into();
            }
            Err(_) => match current_dir() {
                Ok(value) => {
                    self.Virtual_root_path = value.to_str().unwrap().into();
                }
                Err(_) => {
                    return Err(());
                }
            },
        }

        let mut Xila_directory = Generics::Path_type::New();
        Xila_directory.Append("Xila");
        self.Virtual_root_path += Xila_directory;

        if !Path::new(&self.Virtual_root_path.To_str()).exists() {
            match create_dir(&self.Virtual_root_path.To_str()) {
                Ok(_) => {}
                Err(_) => {
                    return Err(());
                }
            }
        }

        Ok(())
    }

    fn Exists(&self, Path: &Generics::Path_type) -> Result<bool, ()> {
        Path::new(&self.Get_full_path(Path).To_str())
            .try_exists()
            .map_err(|_| ())
    }

    fn Open_file(
        &self,
        Path: &Generics::Path_type,
        Mode: Generics::Mode_type,
    ) -> Result<Generics::File_type, ()> {
        let Full_path = self.Get_full_path(Path);
        let Full_path = Full_path.To_str();

        let File = match Mode {
            Generics::Mode_type::Read => File::open(Full_path).map_err(|_| ())?,
            Generics::Mode_type::Write => File::create(Full_path).map_err(|_| ())?,
            Generics::Mode_type::Read_write => OpenOptions::new()
                .read(true)
                .write(true)
                .open(Full_path)
                .map_err(|_| ())?,
            Generics::Mode_type::Append => OpenOptions::new()
                .append(true)
                .open(Full_path)
                .map_err(|_| ())?,
            Generics::Mode_type::Read_append => OpenOptions::new()
                .read(true)
                .append(true)
                .open(Full_path)
                .map_err(|_| ())?,
        };

        let File_identifier = self.Register_file(File).map_err(|_| ())?;

        Ok(Generics::File_type::New(File_identifier, self))
    }

    fn Read_file(
        &self,
        File_identifier: Generics::File_identifier_type,
        Buffer: &mut [u8],
    ) -> Result<usize, ()> {
        let mut Open_files = self.Open_files.write().unwrap();
        let File = match Open_files.get_mut(&File_identifier) {
            Some(File) => File,
            None => return Err(()),
        };
        Ok(File.read(Buffer).map_err(|_| ())?)
    }

    fn Write_file(
        &self,
        File_identifier: Generics::File_identifier_type,
        Buffer: &[u8],
    ) -> Result<usize, ()> {
        let mut Open_files = self.Open_files.write().unwrap();
        let File = match Open_files.get_mut(&File_identifier) {
            Some(File) => File,
            None => return Err(()),
        };
        File.write(Buffer).map_err(|_| ())
    }

    fn Flush_file(&self, File_identifier: Generics::File_identifier_type) -> Result<(), ()> {
        let mut Open_files = self.Open_files.write().unwrap();
        let File = match Open_files.get_mut(&File_identifier) {
            Some(File) => File,
            None => return Err(()),
        };
        File.flush().map_err(|_| ())
    }

    fn Close_file(&self, File_identifier: Generics::File_identifier_type) -> Result<(), ()> {
        self.Unregister_file(File_identifier)
    }

    fn Get_file_type(
        &self,
        File: Generics::File_identifier_type,
    ) -> Result<Generics::Type_type, ()> {
        let mut Open_files = self.Open_files.write().unwrap();
        let File = match Open_files.get_mut(&File) {
            Some(File) => File,
            None => return Err(()),
        };

        match File.metadata() {
            Ok(metadata) => Ok(metadata.file_type().into()),
            Err(_) => Err(()),
        }
    }

    fn Get_file_size(
        &self,
        File_identifier: Generics::File_identifier_type,
    ) -> Result<Generics::Size_type, ()> {
        let mut Open_files = self.Open_files.write().unwrap();
        let File = match Open_files.get_mut(&File_identifier) {
            Some(File) => File,
            None => return Err(()),
        };
        match File.metadata() {
            Ok(metadata) => Ok(metadata.len().into()),
            Err(_) => Err(()),
        }
    }

    fn Get_file_position(
        &self,
        File: Generics::File_identifier_type,
    ) -> Result<Generics::Size_type, ()> {
        let mut Open_files = self.Open_files.write().unwrap();
        let File = match Open_files.get_mut(&File) {
            Some(File) => File,
            None => return Err(()),
        };
        match File.seek(std::io::SeekFrom::Current(0)) {
            Ok(Position) => Ok(Position.into()),
            Err(_) => Err(()),
        }
    }

    fn Set_file_position(
        &self,
        File: Generics::File_identifier_type,
        Position_type: Generics::Position_type,
    ) -> Result<Generics::Size_type, ()> {
        let mut Open_files = self.Open_files.write().unwrap();
        let File = match Open_files.get_mut(&File) {
            Some(File) => File,
            None => return Err(()),
        };
        match File.seek(match Position_type {
            Generics::Position_type::Start(Value) => SeekFrom::Start(Value.0),
            Generics::Position_type::Current(Value) => SeekFrom::Current(Value),
            Generics::Position_type::End(Value) => SeekFrom::End(Value),
        }) {
            Ok(Position) => Ok(Position.into()),
            Err(_) => Err(()),
        }
    }

    fn Delete_file(&self, Path: &Generics::Path_type) -> Result<(), ()> {
        remove_file(self.Get_full_path(Path).To_str()).map_err(|_| ())
    }

    fn Create_directory(&self, Path: &Generics::Path_type) -> Result<(), ()> {
        create_dir(self.Get_full_path(Path).To_str()).map_err(|_| ())
    }

    fn Create_directory_recursive(&self, Path: &Generics::Path_type) -> Result<(), ()> {
        create_dir_all(self.Get_full_path(Path).To_str()).map_err(|_| ())
    }

    fn Delete_directory(&self, Path: &Generics::Path_type) -> Result<(), ()> {
        remove_dir(self.Get_full_path(Path).To_str()).map_err(|_| ())
    }

    fn Delete_directory_recursive(&self, Path: &Generics::Path_type) -> Result<(), ()> {
        remove_dir_all(self.Get_full_path(Path).To_str()).map_err(|_| ())
    }

    fn Move(
        &self,
        Path: &Generics::Path_type,
        Destination: &Generics::Path_type,
    ) -> Result<(), ()> {
        rename(
            self.Get_full_path(Path).To_str(),
            self.Get_full_path(Destination).To_str(),
        )
        .map_err(|_| ())
    }
}

impl From<FileType> for Generics::Type_type {
    fn from(value: FileType) -> Self {
        if value.is_dir() {
            return Generics::Type_type::Directory;
        } else if value.is_symlink() {
            return Generics::Type_type::Symbolic_link;
        }
        Generics::Type_type::File
    }
}

// - Test
#[cfg(test)]
mod tests {
    use super::{Generics::*, *};
    use std::fs::File as STD_File;
    use std::path::Path as STD_Path;

    const Test_directory_path: &str = "Test";

    fn Get_path_in_test(Path: &Generics::Path_type) -> Generics::Path_type {
        Generics::Path_type::from(Test_directory_path) + Path
    }

    fn Reset_test_directory(File_system: &File_system_type) {
        let Test_directory_full_path =
            File_system.Get_full_path(&Generics::Path_type::from(Test_directory_path));
        if !STD_Path::new(&Test_directory_full_path.To_str()).exists() {
            create_dir(&Test_directory_full_path.To_str()).unwrap();
        }
    }

    #[test]
    fn Exists() {
        let mut File_system = File_system_type::New();
        assert!(File_system.Initialize().is_ok());
        Reset_test_directory(&File_system);
        let File_path = Get_path_in_test(&Path_type::from("exists.txt"));
        assert!(!File_system.Exists(&File_path).unwrap());
        let mut File = STD_File::create(File_system.Get_full_path(&File_path).To_str()).unwrap();
        assert!(File.write_all(b"Hello, world!").is_ok());
        assert!(File_system.Exists(&File_path).unwrap());
        assert!(remove_file(File_system.Get_full_path(&File_path).To_str()).is_ok());
        assert!(!File_system.Exists(&File_path).unwrap());
    }

    #[test]
    fn File_manipulation() {
        let mut File_system = File_system_type::New();
        assert!(File_system.Initialize().is_ok());
        Reset_test_directory(&File_system);

        let File_path = Get_path_in_test(&Path_type::from("delete_file.txt"));
        assert!(!File_system.Exists(&File_path).unwrap());
        assert!(File_system.Open_file(&File_path, Mode_type::Write).is_ok());
        assert!(File_system.Exists(&File_path).unwrap());
        assert!(File_system.Delete_file(&File_path).is_ok());
        assert!(!File_system.Exists(&File_path).unwrap());
    }

    #[test]
    fn Directory_operations() {
        let mut File_system = File_system_type::New();
        assert!(File_system.Initialize().is_ok());

        let Directory_path = Get_path_in_test(&Path_type::from("directory"));
        assert!(!File_system.Exists(&Directory_path).unwrap());
        assert!(File_system.Create_directory(&Directory_path).is_ok());
        assert!(File_system.Exists(&Directory_path).unwrap());
        assert!(File_system.Delete_directory(&Directory_path).is_ok());
        assert!(!File_system.Exists(&Directory_path).unwrap());
    }

    #[test]
    fn File_operations() {
        let mut File_system = File_system_type::New();
        assert!(File_system.Initialize().is_ok());
        Reset_test_directory(&File_system);

        let File_path = Get_path_in_test(&Path_type::from("file_operations.txt"));
        assert!(!File_system.Exists(&File_path).unwrap());
        let mut File = File_system.Open_file(&File_path, Mode_type::Write).unwrap();
        assert!(File.write_all(b"Hello, world!").is_ok());
        assert!(File_system.Exists(&File_path).unwrap());
        assert!(File_system.Delete_file(&File_path).is_ok());
        assert!(!File_system.Exists(&File_path).unwrap());
    }

    #[test]
    fn File_metadata() {
        let mut File_system = File_system_type::New();
        assert!(File_system.Initialize().is_ok());
        Reset_test_directory(&File_system);

        let File_path = Get_path_in_test(&Path_type::from("file_metadata.txt"));
        assert!(!File_system.Exists(&File_path).unwrap());
        let mut File = File_system.Open_file(&File_path, Mode_type::Write).unwrap();
        assert!(File.write_all(b"Hello, world!").is_ok());
        assert!(File_system.Exists(&File_path).unwrap());
        assert!(File.Get_size().unwrap() == Size_type(13));
        assert!(File_system.Delete_file(&File_path).is_ok());
        assert!(!File_system.Exists(&File_path).unwrap());
    }
}
