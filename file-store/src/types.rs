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

//! The main types used in this crate.
pub(crate) mod error;
pub(crate) mod future;
pub(crate) mod objects;
pub(crate) mod path;
pub(crate) mod stream;

use std::io;

use bytes::buf::Buf;
use bytes::{Bytes, IntoBuf};
use futures::executor::{block_on_stream, BlockingStream};
use futures::stream::Stream;

use super::FileStore;
pub use error::{StorageError, StorageErrorKind, StorageResult, TransferError};
pub use future::WrappedFuture;
pub use objects::{Object, ObjectInfo, ObjectType, UploadInfo};
pub use path::ObjectPath;
pub use stream::WrappedStream;

/// The data type used for streaming data from and to files.
pub type Data = Bytes;

/// A stream that returns [`Data`](type.Data.html).
pub type DataStream = WrappedStream<StorageResult<Data>>;
/// A future that returns a connected [`FileStore`](enum.FileStore.html) implementation.
pub type ConnectFuture = WrappedFuture<StorageResult<FileStore>>;
/// A stream that returns [`Object`s](enum.Object.html).
pub type ObjectStream = WrappedStream<StorageResult<Object>>;
/// A future that returns an [`ObjectStream`](type.ObjectStream.html).
pub type ObjectStreamFuture = WrappedFuture<StorageResult<ObjectStream>>;
/// A future that returns an [`Object`](enum.Object.html).
pub type ObjectFuture = WrappedFuture<StorageResult<Object>>;
/// A future that resolves whenever the requested operation is complete.
pub type OperationCompleteFuture = WrappedFuture<StorageResult<()>>;
/// A future that resolves when a write operation is complete.
pub type WriteCompleteFuture = WrappedFuture<Result<(), TransferError>>;
/// A future that resolves to a [`DataStream`](type.DataStream.html).
pub type DataStreamFuture = WrappedFuture<StorageResult<DataStream>>;
/// A future that resolves when the copy is complete.
pub type CopyCompleteFuture = WrappedFuture<Result<(), TransferError>>;
/// A future that resolves when the move is complete.
pub type MoveCompleteFuture = WrappedFuture<Result<(), TransferError>>;

pub(crate) struct BlockingStreamReader<S>
where
    S: Unpin + Stream,
{
    stream: BlockingStream<S>,
    reader: Option<Box<dyn io::Read>>,
}

impl<S> BlockingStreamReader<S>
where
    S: Unpin + Stream,
{
    pub fn from_stream(stream: S) -> BlockingStreamReader<S> {
        BlockingStreamReader {
            stream: block_on_stream(stream),
            reader: None,
        }
    }
}

impl<S, O, E> io::Read for BlockingStreamReader<S>
where
    S: Unpin + Stream<Item = Result<O, E>>,
    O: IntoBuf,
    <O as IntoBuf>::Buf: 'static,
    E: Into<StorageError>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.reader {
            Some(ref mut r) => r.read(buf),
            None => match self.stream.next() {
                Some(Ok(d)) => {
                    let mut reader = d.into_buf().reader();
                    let count = reader.read(buf)?;
                    self.reader = Some(Box::new(reader));
                    Ok(count)
                }
                Some(Err(e)) => Err(e.into().into()),
                None => Ok(0),
            },
        }
    }
}
