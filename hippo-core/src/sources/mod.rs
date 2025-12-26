//! Source connectors for different storage providers
//!
//! Currently only local file system sources are supported.
//! Cloud integrations (Google Drive, iCloud, S3, Dropbox) are planned for future releases.
//!
//! Local sources are handled directly in the indexer module using the `Source::Local` variant.

// Future cloud integrations will be implemented here as submodules:
// - google_drive: OAuth 2.0 integration with Google Drive API
// - icloud: Apple CloudKit integration
// - s3: AWS S3 bucket access
// - dropbox: Dropbox API integration
