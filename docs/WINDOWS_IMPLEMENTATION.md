# Windows Implementation Details

The Rust Collector tool now includes a fully-implemented Windows-specific module that enables collection of locked system files using the Windows Backup API.

## Key Features

1. **Privilege Elevation**
   - Automatically acquires necessary Windows privileges:
     - SeBackupPrivilege
     - SeRestorePrivilege
     - SeSecurityPrivilege 
     - SeTakeOwnershipPrivilege
     - SeDebugPrivilege
   - Properly handles privilege acquisition failures

2. **Raw File Access**
   - Uses Windows backup semantics to access locked system files
   - Handles both files and directories recursively
   - Preserves file metadata (creation, access, modification times)
   - Reports on files that couldn't be fully accessed due to locks

3. **Cross-Platform Development**
   - Provides mock implementations for non-Windows platforms
   - Conditional compilation for platform-specific code
   - Allows development and testing on macOS/Linux

## Technical Implementation

### Privilege Handling

The Windows privilege system is implemented in `privileges.rs`, which:

- Opens the current process token
- Looks up and enables necessary privileges
- Tracks and reports success/failure for each privilege
- Continues collection even with partial privilege acquisition

### File Access

Raw file access is implemented in `raw_access.rs`, which:

- Uses CreateFileW with FILE_FLAG_BACKUP_SEMANTICS
- Handles both individual files and recursive directory collection
- Properly handles wide character paths (UCS-2/UTF-16)
- Efficiently reads files in chunks using a 1MB buffer

### File and Directory Metadata

The collector captures complete file metadata:

- Original path
- File size
- Creation time
- Last access time
- Last modification time
- Lock status
- Collection timestamp

### Error Handling

The implementation includes robust error handling:

- Continues collection even when some files fail
- Clearly reports locked or inaccessible files
- Provides detailed error information
- Tracks partial success for required artifacts

## Compilation Notes

When building on Windows, the following features are required:

```toml
winapi = { version = "0.3", features = [
    "basetsd", "errhandlingapi", "fileapi", "handleapi", "minwindef", 
    "processthreadsapi", "securitybaseapi", "winbase", "winnt", "minwinbase", 
    "wincrypt", "winerror", "ntdef", "sysinfoapi", "timezoneapi", 
    "memoryapi", "ioapiset", "synchapi"
]}
```

## Usage Requirements

To use the collector with full capabilities on Windows:

1. Run as Administrator (for privilege elevation)
2. Modern Windows versions (Windows 7/Server 2008 R2 or newer)
3. Sufficient disk space for collected artifacts