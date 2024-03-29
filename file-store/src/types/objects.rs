// Copyright 2019 Dave Townsend
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Object types.

use std::cmp::{Ordering, PartialOrd};
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::time::SystemTime;

use enum_dispatch::enum_dispatch;

use super::*;
use crate::backends::b2::B2Object;
use crate::backends::file::FileObject;

/// An object's type. For most backends this will just be File.
///
/// This crate really only deals with file manipulations and most backends only
/// support files (in some cases called objects). Things like directories often
/// don't really exist. In some cases though backends do have real directories
/// and symlinks and would not support creating a file of the same name without
/// removing them first.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ObjectType {
    /// A regular file.
    File,
    /// A physical directory.
    Directory,
    /// A symbolic link.
    Symlink,
    /// An physical object of unknown type.
    Unknown,
}

impl Eq for ObjectType {}

impl PartialOrd for ObjectType {
    fn partial_cmp(&self, other: &ObjectType) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ObjectType {
    fn cmp(&self, other: &ObjectType) -> Ordering {
        if self == other {
            return Ordering::Equal;
        }

        // Values are not exual at this point.
        match other {
            // Directories are always sorted earlier.
            ObjectType::Directory => return Ordering::Greater,
            // Unknowns are always sorted later.
            ObjectType::Unknown => return Ordering::Less,
            _ => (),
        }

        match self {
            // Directories are always sorted earlier.
            ObjectType::Directory => Ordering::Less,
            // Unknowns are always sorted later.
            ObjectType::Unknown => Ordering::Greater,
            // Other must be a symlink here.
            ObjectType::File => Ordering::Less,
            // Other myst be a file here.
            ObjectType::Symlink => Ordering::Greater,
        }
    }
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ObjectType::File => f.pad("file"),
            ObjectType::Directory => f.pad("dir"),
            ObjectType::Symlink => f.pad("symlink"),
            ObjectType::Unknown => f.pad("unknown"),
        }
    }
}

#[enum_dispatch(ObjectInfo)]
/// An object of some kind that exists at a path in the storage system.
///
/// Most backends only support File objects, and this crate only really supports
/// manipulating file objects. This type does however support the idea of a non
/// file type that physically exists at a path.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum Object {
    B2(B2Object),
    File(FileObject),
}

impl PartialEq for Object {
    fn eq(&self, other: &Object) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Object {}

impl PartialOrd for Object {
    fn partial_cmp(&self, other: &Object) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Object {
    fn cmp(&self, other: &Object) -> Ordering {
        let order = self.path().cmp(&other.path());
        if order != Ordering::Equal {
            return order;
        }

        self.len().cmp(&other.len())
    }
}

/// Information about an object currently stored in a backend storage system.
///
/// Some of the information is optional because not all storage backends can get
/// access to it.
#[enum_dispatch]
pub trait ObjectInfo {
    /// Gets the object's path.
    fn path(&self) -> ObjectPath;

    /// Gets the object's size in bytes.
    fn len(&self) -> u64;

    /// Checks if the object is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Gets the object's type.
    fn object_type(&self) -> ObjectType;

    /// Gets the last modification time for the object.
    fn modified(&self) -> Option<SystemTime>;

    /// Creates an [`UploadInfo`](struct.UploadInfo.html) for uploading this
    /// object to a new path.
    fn as_upload<P>(&self, path: P) -> StorageResult<UploadInfo>
    where
        Self: Clone,
        P: TryInto<ObjectPath>,
        StorageError: From<P::Error>,
    {
        let path = path.try_into()?;
        let mut info = UploadInfo::from(self.clone());
        info.path = path;
        Ok(info)
    }
}

/// Information used to upload a file.
///
/// This allows attempting to set various properties of a file on upload. Not
/// all backends will support setting them all. Optional properties will just
/// use the backend's default if unset.
///
/// Both [`ObjectPath`](struct.ObjectPath.html) and [`Object`](enum.Object.html)
/// have `Into` implementations for this object so you may not need to create
/// one of these manually enless there are specific properties you wish to
/// change.
#[derive(Clone, Debug)]
pub struct UploadInfo {
    /// The path to upload to.
    pub path: ObjectPath,
    /// Sets the last modified time for the file.
    pub modified: Option<SystemTime>,
}

impl<I> From<I> for UploadInfo
where
    I: ObjectInfo,
{
    fn from(info: I) -> UploadInfo {
        UploadInfo {
            path: info.path(),
            modified: info.modified(),
        }
    }
}

impl From<ObjectPath> for UploadInfo {
    fn from(path: ObjectPath) -> UploadInfo {
        UploadInfo {
            path,
            modified: None,
        }
    }
}

impl TryFrom<&str> for UploadInfo {
    type Error = error::StorageError;

    fn try_from(s: &str) -> Result<UploadInfo, error::StorageError> {
        Ok(ObjectPath::new(s)?.into())
    }
}
