use crate::Prelude::{
    Error_type, File_identifier_type, File_system_traits, Flags_type, Path_owned_type, Path_type,
    Permissions_type, Position_type, Result, Size_type, Type_type,
};

use std::collections::HashMap;
use std::env::{current_dir, var};
use std::fs::*;
use std::io::{ErrorKind, Read, Seek, Write};

use std::path::PathBuf;

use Task::Task_identifier_type;

impl From<FileType> for Type_type {
    fn from(value: FileType) -> Self {
        if value.is_dir() {
            return Type_type::Directory;
        } else if value.is_symlink() {
            return Type_type::Symbolic_link;
        }
        Type_type::File
    }
}

impl From<std::io::ErrorKind> for Error_type {
    fn from(Error: std::io::ErrorKind) -> Self {
        use std::io::ErrorKind;

        match Error {
            ErrorKind::PermissionDenied => Error_type::Permission_denied,
            ErrorKind::NotFound => Error_type::Not_found,
            ErrorKind::AlreadyExists => Error_type::Already_exists,
            ErrorKind::InvalidInput => Error_type::Invalid_path,
            ErrorKind::InvalidData => Error_type::Invalid_file,
            _ => Error_type::Unknown,
        }
    }
}

impl From<std::io::Error> for Error_type {
    fn from(Error: std::io::Error) -> Self {
        Error.kind().into()
    }
}

impl Flags_type {
    fn Into_open_options(self, Open_options: &mut OpenOptions) {
        Open_options
            .read(self.Get_mode().Get_read())
            .write(self.Get_mode().Get_write() || self.Get_status().Get_append());
    }
}

impl From<&PathBuf> for Path_owned_type {
    fn from(item: &PathBuf) -> Self {
        Path_owned_type::New(item.to_str().unwrap().to_string()).unwrap()
    }
}

#[cfg(target_family = "unix")]
impl From<&Permissions_type> for std::fs::Permissions {
    fn from(Permissions: &Permissions_type) -> Self {
        use std::os::unix::fs::PermissionsExt;

        std::fs::Permissions::from_mode(Permissions.To_unix() as u32)
    }
}

pub struct File_system_type {
    Virtual_root_path: Path_owned_type,
    Open_files: HashMap<u32, File>,
}

impl File_system_type {
    pub fn New() -> Result<Self> {
        Ok(File_system_type {
            Virtual_root_path: Self::Get_root_path().ok_or(Error_type::Unknown)?,
            Open_files: HashMap::new(),
        })
    }

    fn Get_root_path() -> Option<Path_owned_type> {
        let Root_path = match var("Xila_virtual_root_path") {
            Ok(value) => value,
            Err(_) => match current_dir() {
                Ok(value) => value.to_str()?.to_string(),
                Err(_) => {
                    return None;
                }
            },
        };

        let Root_path = Path_owned_type::try_from(Root_path).ok()?.Append("Xila")?;

        match create_dir(Root_path.as_ref() as &Path_type) {
            Ok(_) => {}
            Err(Error) => {
                if ErrorKind::AlreadyExists != Error.kind() {
                    return None;
                }
            }
        }

        Some(Root_path)
    }

    fn Get_new_file_identifier(
        &self,
        Task_identifier: Task_identifier_type,
    ) -> Option<File_identifier_type> {
        let Start = Self::Get_local_file_identifier(Task_identifier, File_identifier_type::from(0));
        let End =
            Self::Get_local_file_identifier(Task_identifier, File_identifier_type::from(0xFFFF));

        for i in Start..End {
            if !self.Open_files.contains_key(&i) {
                return Some(File_identifier_type::from(i as u16));
            }
        }

        None
    }

    pub fn Get_full_path(&self, Path: &dyn AsRef<Path_type>) -> Result<Path_owned_type> {
        self.Virtual_root_path
            .clone()
            .Join(Path)
            .ok_or(Error_type::Invalid_path)
    }
}

impl File_system_traits for File_system_type {
    fn Exists(&self, Path: &dyn AsRef<Path_type>) -> Result<bool> {
        metadata(self.Get_full_path(&Path)?.as_ref() as &Path_type)
            .map(|_| true)
            .or_else(|Error| match Error.kind() {
                ErrorKind::NotFound => Ok(false),
                _ => Err(Error.kind().into()),
            })
    }

    fn Open(
        &mut self,
        Task_identifier: Task_identifier_type,
        Path: &dyn AsRef<Path_type>,
        Flags: Flags_type,
    ) -> Result<File_identifier_type> {
        let Full_path = self.Get_full_path(&Path)?;

        let mut Open_options = OpenOptions::new();

        Flags.Into_open_options(&mut Open_options);

        let File = Open_options
            .open(Full_path.as_ref() as &Path_type)
            .map_err(|Error| Error.kind())?;

        let File_identifier = self
            .Get_new_file_identifier(Task_identifier)
            .ok_or(Error_type::Too_many_open_files)?;

        let Local_file_identifier =
            Self::Get_local_file_identifier(Task_identifier, File_identifier);

        if self
            .Open_files
            .insert(Local_file_identifier, File)
            .is_some()
        {
            return Err(Error_type::Internal_error);
        }

        Ok(File_identifier)
    }

    fn Read(
        &mut self,
        Task_identifier: Task_identifier_type,
        File_identifier: File_identifier_type,
        Buffer: &mut [u8],
    ) -> Result<Size_type> {
        let Local_file_identifier =
            Self::Get_local_file_identifier(Task_identifier, File_identifier);

        let File = self
            .Open_files
            .get_mut(&Local_file_identifier)
            .ok_or(Error_type::Invalid_identifier)?;

        Ok(File.read(Buffer)?.into())
    }

    fn Write(
        &mut self,
        Task_identifier: Task_identifier_type,
        File_identifier: File_identifier_type,
        Buffer: &[u8],
    ) -> Result<Size_type> {
        let Local_file_identifier =
            Self::Get_local_file_identifier(Task_identifier, File_identifier);

        let File = self
            .Open_files
            .get_mut(&Local_file_identifier)
            .ok_or(Error_type::Invalid_identifier)?;

        Ok(File.write(Buffer)?.into())
    }

    fn Flush(&mut self, Task: Task_identifier_type, File: File_identifier_type) -> Result<()> {
        let Local_file_identifier = Self::Get_local_file_identifier(Task, File);
        let File = self
            .Open_files
            .get_mut(&Local_file_identifier)
            .ok_or(Error_type::Invalid_identifier)?;
        File.flush().map_err(|Error| Error.kind().into())
    }

    fn Close(&mut self, Task: Task_identifier_type, File: File_identifier_type) -> Result<()> {
        let Local_file_identifier = Self::Get_local_file_identifier(Task, File);
        self.Open_files
            .remove(&Local_file_identifier)
            .ok_or(Error_type::Invalid_identifier)?;
        Ok(())
    }

    fn Get_type(&self, _: Task_identifier_type, Path: &dyn AsRef<Path_type>) -> Result<Type_type> {
        let Full_path = self.Get_full_path(&Path)?;
        let Metadata = metadata(Full_path.as_ref() as &Path_type).map_err(|Error| Error.kind())?;
        Ok(Metadata.file_type().into())
    }

    fn Get_size(&self, _: Task_identifier_type, Path: &dyn AsRef<Path_type>) -> Result<Size_type> {
        let Full_path = self.Get_full_path(&Path)?;
        let Metadata = metadata(Full_path.as_ref() as &Path_type).map_err(|Error| Error.kind())?;
        Ok(Metadata.len().into())
    }

    fn Set_position(
        &mut self,
        Task_identifier: Task_identifier_type,
        File_identifier: File_identifier_type,
        Position_type: &Position_type,
    ) -> Result<Size_type> {
        let Local_file_identifier =
            Self::Get_local_file_identifier(Task_identifier, File_identifier);
        let File = match self.Open_files.get_mut(&Local_file_identifier) {
            Some(File) => File,
            None => return Err(Error_type::Invalid_identifier),
        };

        Ok(File
            .seek((*Position_type).into())
            .map_err(|Error| Error_type::from(Error.kind()))?
            .into())
    }

    fn Delete(&mut self, Path: &dyn AsRef<Path_type>) -> Result<()> {
        let Full_path = self.Get_full_path(&Path)?;

        remove_file(Full_path.as_ref() as &Path_type).map_err(|Error| Error.kind().into())
    }

    fn Create_directory(
        &mut self,
        _: Task_identifier_type,
        Path: &dyn AsRef<Path_type>,
    ) -> Result<()> {
        let Full_path = self.Get_full_path(&Path)?;

        create_dir(Full_path.as_ref() as &Path_type).map_err(|Error| Error.kind().into())
    }

    fn Create_file(&mut self, _: Task_identifier_type, Path: &dyn AsRef<Path_type>) -> Result<()> {
        let Full_path = self.Get_full_path(&Path)?;

        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(Full_path.as_ref() as &Path_type)
            .map_err(|Error| Error.kind())?;

        Ok(())
    }

    fn Close_all(&mut self, Task_identifier: Task_identifier_type) -> Result<()> {
        let Start = Self::Get_local_file_identifier(Task_identifier, File_identifier_type::from(0));
        let End =
            Self::Get_local_file_identifier(Task_identifier, File_identifier_type::from(0xFFFF));

        self.Open_files
            .retain(|File_identifier, _| *File_identifier < Start || *File_identifier > End);

        Ok(())
    }

    fn Transfert_file_identifier(
        &mut self,
        Old_task: Task_identifier_type,
        New_task: Task_identifier_type,
        Old_file_identifier: File_identifier_type,
    ) -> Result<File_identifier_type> {
        let Old_local_file_identifier =
            Self::Get_local_file_identifier(Old_task, Old_file_identifier);
        let New_file_identifier = self
            .Get_new_file_identifier(New_task)
            .ok_or(Error_type::Too_many_open_files)?;
        let New_local_file_identifier =
            Self::Get_local_file_identifier(New_task, New_file_identifier);

        let File = self
            .Open_files
            .remove(&Old_local_file_identifier)
            .ok_or(Error_type::Invalid_identifier)?;

        if self
            .Open_files
            .insert(New_local_file_identifier, File)
            .is_some()
        {
            return Err(Error_type::Internal_error);
        }

        Ok(File_identifier_type::from(New_local_file_identifier as u16))
    }

    fn Move(
        &mut self,
        _: Task_identifier_type,
        Source: &dyn AsRef<Path_type>,
        Destination: &dyn AsRef<Path_type>,
    ) -> Result<()> {
        let Source = self.Get_full_path(Source)?;
        let Destination = self.Get_full_path(Destination)?;

        rename(
            Source.as_ref() as &Path_type,
            Destination.as_ref() as &Path_type,
        )?;
        Ok(())
    }
}

// - Test
#[cfg(test)]
mod Tests {}
