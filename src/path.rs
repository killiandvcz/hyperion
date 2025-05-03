//! Path module for NanoDB
//!
//! This module defines the Path structure, which represents
//! a hierarchical path to a specific data endpoint in the database.

use std::fmt;
use std::str::FromStr;
use thiserror::Error;
use serde::{Serialize, Deserialize};

/// Errors that can occur when working with paths
#[derive(Error, Debug, PartialEq)]
pub enum PathError {
    #[error("Invalid path format: {0}")]
    InvalidFormat(String),
    #[error("Empty path")]
    EmptyPath,
}

/// Types of path segments
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SegmentType {
    /// Regular named segment
    Named(String),
    /// Single-level wildcard (*)
    SingleWildcard,
    /// Multi-level wildcard (**)
    MultiWildcard,
    /// Array index segment (e.g., [0])
    ArrayIndex(usize),
}

/// A segment in a path
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PathSegment(SegmentType);

impl PathSegment {
    /// Create a new path segment
    pub fn new<S: Into<String>>(segment: S) -> Self {
        let segment_str = segment.into();
        
        // Check if this is a wildcard
        if segment_str == "*" {
            return PathSegment(SegmentType::SingleWildcard);
        } else if segment_str == "**" {
            return PathSegment(SegmentType::MultiWildcard);
        }
        
        // Check if this is an array index
        if let Some(index_str) = segment_str.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            if let Ok(index) = index_str.parse::<usize>() {
                return PathSegment(SegmentType::ArrayIndex(index));
            }
        }
        
        // Regular named segment
        PathSegment(SegmentType::Named(segment_str))
    }
    
    /// Get the segment as a string reference
    pub fn as_str(&self) -> String {
        match &self.0 {
            SegmentType::Named(name) => name.clone(),
            SegmentType::SingleWildcard => "*".to_string(),
            SegmentType::MultiWildcard => "**".to_string(),
            SegmentType::ArrayIndex(idx) => format!("[{}]", idx),
        }
    }
    
    /// Check if this segment is a single-level wildcard
    pub fn is_single_wildcard(&self) -> bool {
        matches!(self.0, SegmentType::SingleWildcard)
    }
    
    /// Check if this segment is a multi-level wildcard
    pub fn is_multi_wildcard(&self) -> bool {
        matches!(self.0, SegmentType::MultiWildcard)
    }
    
    /// Check if this segment is any kind of wildcard
    pub fn is_wildcard(&self) -> bool {
        self.is_single_wildcard() || self.is_multi_wildcard()
    }
    
    /// Check if this segment is an array index
    pub fn is_array_index(&self) -> bool {
        matches!(self.0, SegmentType::ArrayIndex(_))
    }
    
    /// Get the array index if this is an array index segment
    pub fn as_index(&self) -> Option<usize> {
        match self.0 {
            SegmentType::ArrayIndex(idx) => Some(idx),
            _ => None,
        }
    }
    
    /// Check if this segment matches another segment
    /// (including wildcard matching)
    pub fn matches(&self, other: &PathSegment) -> bool {
        match &self.0 {
            // A single wildcard matches any single segment
            SegmentType::SingleWildcard => true,
            
            // Multi-wildcard should not be used for single segment matching
            SegmentType::MultiWildcard => true,
            
            // Named segments match if they have the same name
            SegmentType::Named(name) => {
                match &other.0 {
                    SegmentType::Named(other_name) => name == other_name,
                    _ => false,
                }
            },
            
            // Array indices match if they have the same index
            SegmentType::ArrayIndex(idx) => {
                match &other.0 {
                    SegmentType::ArrayIndex(other_idx) => idx == other_idx,
                    _ => false,
                }
            },
        }
    }
}

/// A path in the database (e.g., "users.u-123456.profile.bio")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Path {
    segments: Vec<PathSegment>,
}

impl Path {
    /// Create a new empty path
    pub fn new() -> Self {
        Path { segments: Vec::new() }
    }
    
    /// Create a path from a vector of segments
    pub fn from_segments(segments: Vec<PathSegment>) -> Self {
        Path { segments }
    }
    
    /// Add a segment to the path
    pub fn push<S: Into<String>>(&mut self, segment: S) {
        self.segments.push(PathSegment::new(segment));
    }
    
    /// Get the number of segments in the path
    pub fn len(&self) -> usize {
        self.segments.len()
    }
    
    /// Check if the path is empty
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
    
    /// Get all segments in the path
    pub fn segments(&self) -> &[PathSegment] {
        &self.segments
    }
    
    /// Get a specific segment by index
    pub fn segment(&self, index: usize) -> Option<&PathSegment> {
        self.segments.get(index)
    }
    
    /// Check if this path contains wildcards
    pub fn has_wildcards(&self) -> bool {
        self.segments.iter().any(|s| s.is_wildcard())
    }
    
    /// Check if this path starts with the given prefix path
    pub fn starts_with(&self, prefix: &Path) -> bool {
        if prefix.len() > self.len() {
            return false;
        }
        
        for (i, segment) in prefix.segments().iter().enumerate() {
            if !segment.matches(&self.segments[i]) {
                return false;
            }
        }
        
        true
    }
    
    /// Check if this path matches a pattern (which may contain wildcards)
    pub fn matches(&self, pattern: &Path) -> bool {
        // If the pattern is empty, it only matches empty paths
        if pattern.is_empty() {
            return self.is_empty();
        }
        
        // Check if the pattern has a multi-wildcard
        for (i, segment) in pattern.segments().iter().enumerate() {
            if segment.is_multi_wildcard() {
                // A multi-wildcard can match zero or more segments

                // If it's the last segment, the multi-wildcard matches everything
                if i == pattern.len() - 1 {
                    return true;
                }
                
                // Try to match the rest of the pattern with every possible
                // suffix of the path
                let remaining_pattern = Path::from_segments(pattern.segments()[i+1..].to_vec());
                
                // Try matching the remaining pattern at each position of the remaining path
                for j in i..=self.len() {
                    let suffix = Path::from_segments(self.segments[j..].to_vec());
                    if suffix.matches(&remaining_pattern) {
                        return true;
                    }
                }
                
                return false;
            }
        }
        
        // If there's no multi-wildcard, the pattern and path must have the same length
        // (after accounting for single wildcards)
        if pattern.len() != self.len() {
            return false;
        }
        
        // Check each segment for a match
        for (i, pattern_segment) in pattern.segments().iter().enumerate() {
            if !pattern_segment.matches(&self.segments[i]) {
                return false;
            }
        }
        
        true
    }
}

/// Parse a string into a Path
impl FromStr for Path {
    type Err = PathError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(PathError::EmptyPath);
        }
        
        // Split by dots and create segments
        let segments = s.split('.')
            .map(PathSegment::new)
            .collect();
        
        Ok(Path { segments })
    }
}

/// Format a Path as a string with dot separators
impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path_str = self.segments
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(".");
        
        write!(f, "{}", path_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_path_parsing() {
        let path = "users.u-123456.profile.bio".parse::<Path>().unwrap();
        assert_eq!(path.len(), 4);
        assert_eq!(path.segment(0).unwrap().as_str(), "users");
        assert_eq!(path.segment(1).unwrap().as_str(), "u-123456");
        assert_eq!(path.segment(2).unwrap().as_str(), "profile");
        assert_eq!(path.segment(3).unwrap().as_str(), "bio");
    }
    
    #[test]
    fn test_path_formatting() {
        let mut path = Path::new();
        path.push("users");
        path.push("u-123456");
        path.push("profile");
        path.push("bio");
        
        assert_eq!(path.to_string(), "users.u-123456.profile.bio");
    }
    
    #[test]
    fn test_starts_with() {
        let full_path: Path = "users.u-123456.profile.bio".parse().unwrap();
        let prefix: Path = "users.u-123456".parse().unwrap();
        
        assert!(full_path.starts_with(&prefix));
    }
    
    #[test]
    fn test_wildcard_parsing() {
        let path = "users.*.profile.bio".parse::<Path>().unwrap();
        assert_eq!(path.len(), 4);
        assert!(path.segment(1).unwrap().is_single_wildcard());
        
        let path2 = "users.**.bio".parse::<Path>().unwrap();
        assert_eq!(path2.len(), 3);
        assert!(path2.segment(1).unwrap().is_multi_wildcard());
    }
    
    #[test]
    fn test_path_matching_single_wildcard() {
        let pattern: Path = "users.*.email".parse().unwrap();
        let path1: Path = "users.u-123456.email".parse().unwrap();
        let path2: Path = "users.u-789012.email".parse().unwrap();
        let path3: Path = "users.u-123456.profile".parse().unwrap();
        
        assert!(path1.matches(&pattern));
        assert!(path2.matches(&pattern));
        assert!(!path3.matches(&pattern));
    }
    
    #[test]
    fn test_path_matching_multi_wildcard() {
        let pattern: Path = "users.**.bio".parse().unwrap();
        let path1: Path = "users.u-123456.bio".parse().unwrap();
        let path2: Path = "users.u-123456.profile.bio".parse().unwrap();
        let path3: Path = "users.u-123456.profile.social.bio".parse().unwrap();
        let path4: Path = "users.u-123456.profile".parse().unwrap();
        
        assert!(path1.matches(&pattern));
        assert!(path2.matches(&pattern));
        assert!(path3.matches(&pattern));
        assert!(!path4.matches(&pattern));
    }
    
    #[test]
    fn test_array_index_parsing() {
        let path = "users.u-123456.tags[0]".parse::<Path>().unwrap();
        assert_eq!(path.len(), 3);
        
        let segment = path.segment(2).unwrap();
        assert!(segment.is_array_index());
        assert_eq!(segment.as_index(), Some(0));
    }
}