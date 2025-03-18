// Copyright (C) 2025 Ryan Daum <ryan.daum@gmail.com> This program is free
// software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, version
// 3.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// this program. If not, see <https://www.gnu.org/licenses/>.
//

use std::borrow::Cow;
use std::path::Path;

/// Represents a client asset file embedded in the binary
#[iftree::include_file_tree("paths = '/crates/web-host/src/client/**'")]
pub struct ClientAsset {
    /// File path relative to the base folder
    pub relative_path: &'static str,
    /// File contents as bytes
    pub contents_bytes: &'static [u8],
    /// File contents as string (if UTF-8)
    pub contents_str: &'static str,
    /// Function to get bytes (reads from disk in debug mode)
    pub get_bytes: fn() -> Cow<'static, [u8]>,
    /// Function to get string (reads from disk in debug mode)
    pub get_str: fn() -> Cow<'static, str>,
}

impl ClientAsset {
    /// Get the file extension (if any)
    pub fn extension(&self) -> Option<&str> {
        Path::new(self.relative_path)
            .extension()
            .and_then(|ext| ext.to_str())
    }

    /// Get the file name without path
    pub fn file_name(&self) -> Option<&str> {
        Path::new(self.relative_path)
            .file_name()
            .and_then(|name| name.to_str())
    }

    /// Get the content type based on file extension
    pub fn content_type(&self) -> &'static str {
        match self.extension() {
            Some("html") => "text/html",
            Some("css") => "text/css",
            Some("js") => "application/javascript",
            Some("ts") => "application/typescript",
            Some("json") => "application/json",
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("svg") => "image/svg+xml",
            Some("ico") => "image/x-icon",
            _ => "application/octet-stream",
        }
    }

    /// Find an asset by its relative path
    pub fn find(path: &str) -> Option<&'static ClientAsset> {
        // Normalize the path by removing leading slash
        let normalized_path = path.trim_start_matches('/');
        
        // Look for exact match in client/
        let client_path = format!("crates/web-host/src/client/{}", normalized_path);
        
        ASSETS.iter().find(|asset| asset.relative_path == client_path)
    }
}
