# Security Guide for rs-collector

## Overview

rs-collector is a forensic collection tool that requires elevated privileges to access system artifacts. This document outlines the security considerations, best practices, and safety measures when using rs-collector.

## ⚠️ Security Warnings

**IMPORTANT**: This tool requires administrative/root privileges to collect many system artifacts. Running with elevated privileges carries inherent security risks.

## Required Privileges

### Windows
- **SeDebugPrivilege**: Required for memory access and process information
- **SeBackupPrivilege**: Required for accessing locked files
- **SeRestorePrivilege**: Required for raw disk access
- **Administrator**: Required for Registry and system file access

### Linux/macOS
- **Root access**: Required for:
  - Memory collection (`/dev/mem`, `/proc/kcore`)
  - System log access
  - Process information
  - Raw disk access

## Security Best Practices

### 1. Verify the Binary

Always verify the authenticity of the rs-collector binary before running:

```bash
# Verify SHA256 checksum
sha256sum rs-collector

# Verify GPG signature (if provided)
gpg --verify rs-collector.sig rs-collector
```

### 2. Limit Execution Scope

Use configuration files to limit what the tool collects:

```yaml
# Minimal collection example
artifacts:
  - name: "system_logs"
    artifact_type: Logs
    source_path: "/var/log/syslog"
    required: false
```

### 3. Secure Output Handling

#### Set Restrictive Permissions
```bash
# Create output directory with restricted permissions
mkdir -m 700 /secure/output
rs-collector -o /secure/output
```

#### Encrypt Output Files
```bash
# Encrypt collected data immediately
rs-collector -o - | gpg -e -r forensics@example.com > collection.gpg
```

### 4. Network Security

When using cloud upload features:

#### S3 Upload
- Use IAM roles with minimal permissions
- Enable S3 bucket encryption
- Use HTTPS endpoints only
- Rotate access keys regularly

```yaml
s3_config:
  bucket: "forensics-bucket"
  region: "us-east-1"
  storage_class: "GLACIER"
  server_side_encryption: "AES256"
```

#### SFTP Upload
- Use SSH key authentication (not passwords)
- Verify host keys
- Use non-standard ports if possible
- Limit SFTP user permissions

```yaml
sftp_config:
  host: "sftp.example.com"
  port: 2222
  username: "forensics"
  private_key_path: "/path/to/key"
```

### 5. Operational Security

#### Minimize Runtime
```bash
# Run with specific targets only
rs-collector --config minimal.yaml --no-memory
```

#### Monitor Execution
```bash
# Log all actions
rs-collector -v --log-file collection.log
```

#### Clean Up After Collection
```bash
# Secure deletion of temporary files
shred -vfz /tmp/rs-collector-*
```

## Security Features

### Path Validation
rs-collector validates all file paths to prevent directory traversal attacks:
- Rejects paths containing `..`
- Validates output stays within designated directories
- Sanitizes filenames to prevent injection

### Input Validation
- Configuration files are validated before processing
- Command-line arguments are sanitized
- File sizes are limited to prevent resource exhaustion

### Privilege Management
- Privileges are requested only when needed
- Option to drop privileges after initialization
- Clear logging of privilege escalation

## Sensitive Data Handling

### What rs-collector Collects

Be aware that rs-collector may collect sensitive information including:
- Memory dumps (may contain passwords, keys, sensitive data)
- Browser history and cookies
- System configuration files
- User data and documents
- Network connection information
- Process information

### Protecting Collected Data

1. **Immediate Encryption**
   ```bash
   rs-collector -o collection.zip && \
   gpg -e -r your-key@example.com collection.zip && \
   rm collection.zip
   ```

2. **Secure Transfer**
   - Use encrypted channels (HTTPS, SFTP)
   - Verify endpoint authenticity
   - Use temporary credentials

3. **Access Control**
   - Limit access to collection outputs
   - Use file system encryption
   - Implement audit logging

## Incident Response Considerations

When using rs-collector for incident response:

1. **Maintain Chain of Custody**
   - Document all actions
   - Use write-blockers when possible
   - Generate cryptographic hashes

2. **Minimize System Impact**
   - Run from external media
   - Avoid writing to compromised systems
   - Use streaming output options

3. **Preserve Evidence**
   - Collect volatile data first
   - Document system state
   - Avoid modifying timestamps

## Security Configuration Example

```yaml
# High-security configuration
security:
  validate_paths: true
  encrypt_output: true
  restrictive_permissions: true
  drop_privileges: true
  max_file_size: 1073741824  # 1GB
  blocked_extensions:
    - .key
    - .pem
    - .password
  audit_logging: true
  redact_sensitive_data: true

# Output configuration
output:
  directory: "/secure/forensics"
  compress: true
  encryption_key: "forensics-public-key.pem"
  permissions: 0600
```

## Reporting Security Issues

If you discover a security vulnerability in rs-collector:

1. **Do NOT** create a public issue
2. Email security details to: security@[project-domain]
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fixes (if any)

## Compliance Considerations

When using rs-collector, ensure compliance with:
- Local privacy laws (GDPR, CCPA, etc.)
- Corporate policies
- Legal requirements for evidence handling
- Data retention policies

## Audit Trail

rs-collector can generate detailed audit logs:

```bash
# Enable audit logging
rs-collector --audit-log audit.json

# Audit log includes:
# - All files accessed
# - Privileges requested
# - Configuration used
# - Errors encountered
# - Timestamps for all operations
```

## Conclusion

Security is a critical consideration when using forensic collection tools. Always:
- Understand what data you're collecting
- Protect collected data appropriately
- Follow legal and policy requirements
- Maintain proper documentation
- Use the principle of least privilege

For additional security guidance, consult your organization's security team or forensic procedures.