# Privacy Policy

**Effective Date: January 1, 2025**  
**Last Updated: January 1, 2025**

## Introduction

Ariata ("we," "our," or "the app") is committed to protecting your privacy. This Privacy Policy explains how our iOS application collects, uses, stores, and protects your personal information. Ariata is designed as a personal data ecosystem that YOU control - all data is stored on your own server infrastructure.

## Key Privacy Features

- **You Own Your Data**: All data is transmitted directly to your self-hosted server
- **No Third-Party Analytics**: We don't use any tracking or analytics services
- **No Advertising**: We never use your data for advertising purposes
- **Local Processing First**: Data is collected locally and sent only to your specified endpoint
- **End-to-End Control**: You maintain complete control over data retention and deletion

## Information We Collect

### 1. Location Data

- **What**: Precise GPS coordinates, altitude, speed, and movement patterns
- **When**: Continuously every 10 seconds when the app has permission
- **Why**: To create a personal timeline and location history on your server
- **Permission**: Requires "Always Allow" location permission

### 2. Health & Fitness Data

- **What**:
  - Heart rate and heart rate variability
  - Step count and distance traveled
  - Active energy burned
  - Sleep analysis data
  - Resting heart rate
- **When**: Synchronized every 5 minutes from Apple Health
- **Why**: To provide comprehensive health insights on your personal server
- **Permission**: Requires HealthKit authorization for each data type

### 3. Audio Data

- **What**: 30-second audio recordings with 2-second overlaps
- **When**: Continuously when audio recording is enabled
- **Why**: For transcription and audio journaling on your server
- **Permission**: Requires microphone access
- **Format**: Compressed AAC format at 16kHz sample rate

### 4. Device Information

- **What**:
  - Device model and iOS version
  - App version
  - Unique device identifier (generated locally)
  - Network connection status
- **When**: During app initialization and troubleshooting
- **Why**: To ensure compatibility and assist with technical support

## How We Use Your Information

### Direct Transmission Only

All collected data is transmitted directly to the server endpoint YOU configure during app setup. We do not:

- Store your data on our servers
- Access your personal data
- Share data with third parties
- Use data for marketing or advertising
- Perform analytics on your data

### Local Processing

The app performs minimal processing on your device:

- Temporary SQLite storage before upload
- Data batching for efficient transmission
- Compression of audio data
- Failed upload retry management

## Data Storage

### On Your Device

- **Temporary Buffer**: SQLite database stores data temporarily until uploaded
- **Retention**: Successfully uploaded data is deleted after 3 days
- **Failed Uploads**: Retained for up to 7 days with retry attempts
- **Storage Management**: Automatic cleanup when device storage is low

### On Your Server

- **Full Control**: You determine retention policies on your server
- **Location**: Your self-hosted infrastructure (not our servers)
- **Security**: You implement security measures on your infrastructure
- **Access**: Only you and those you authorize can access your data

## Data Sharing

We do NOT share your personal data with anyone. Specifically:

- **No Third-Party Services**: No analytics, crash reporting, or advertising SDKs
- **No Cloud Backups**: Data is not included in iCloud backups
- **No Transfer**: We never transfer data to our servers or third parties
- **Your Server Only**: Data goes exclusively to your configured endpoint

## Your Rights and Choices

### Control Over Collection

- **Start/Stop**: Toggle data collection on/off at any time in Settings
- **Selective Permissions**: Grant or revoke specific permissions in iOS Settings
- **Data Types**: Choose which types of data to collect
- **Audio Input**: Select preferred microphone (iPhone, Bluetooth, wired)

### Data Management

- **Export**: All data is already exported to your server
- **Deletion**: Delete local cache anytime; manage server data directly
- **Access**: View pending upload queue and sync status in the app
- **Portability**: Data is stored in standard formats on your server

### Permission Management

You can revoke permissions at any time through iOS Settings:

- Settings > Privacy & Security > Location Services
- Settings > Privacy & Security > Health
- Settings > Privacy & Security > Microphone

## Data Security

### In Transit

- **Encryption**: HTTPS/TLS encryption for all data transmission
- **Authentication**: API key authentication for your server
- **Integrity**: Checksums ensure data integrity during transfer

### On Device

- **iOS Security**: Leverages iOS built-in security features
- **Sandboxing**: App data isolated from other apps
- **No Sensitive Storage**: API keys stored in iOS Keychain
- **Temporary Only**: Data deleted after successful upload

### Your Responsibility

Since you control the server infrastructure:

- Implement appropriate security measures
- Maintain secure API endpoints
- Configure proper authentication
- Ensure compliance with local regulations

## Children's Privacy

Ariata is not intended for children under 17. We do not knowingly collect personal information from children. The app requires understanding of self-hosted infrastructure and privacy implications.

## International Data Transfers

Data is transmitted directly from your device to YOUR specified server endpoint. You control the geographic location of your server and any international data transfers.

## Legal Compliance

### GDPR (European Users)

If you're in the EU, you have rights under GDPR:

- **Right to Access**: View all data in the app
- **Right to Rectification**: Correct data on your server
- **Right to Erasure**: Delete data from app and your server
- **Right to Portability**: Data is already in portable formats
- **Right to Object**: Stop collection at any time

### CCPA (California Users)

California residents have additional rights:

- **Right to Know**: This policy describes all data collection
- **Right to Delete**: Remove data from app and your server
- **Right to Opt-Out**: We don't sell personal information
- **Non-Discrimination**: No different treatment for exercising rights

### PIPEDA (Canadian Users)

Canadian users have rights under PIPEDA:

- **Consent**: Explicit consent required for data collection
- **Access**: View and correct your personal information
- **Challenge Compliance**: Contact us with privacy concerns

## Changes to This Policy

We may update this Privacy Policy to reflect changes in our app or legal requirements. We will notify you of significant changes through:

- App Store update notes
- In-app notifications
- Update to the "Last Updated" date

## Contact Information

For privacy-related questions or concerns:

**Email**: <privacy@ariata.com>  
**GitHub Issues**: <https://github.com/ariata-os/ariata/issues>  
**Documentation**: <https://docs.ariata.com>

## Data Processing Agreement

For users requiring a Data Processing Agreement (DPA) for compliance purposes, note that:

- You are both the data controller and processor
- The app acts solely as a data transmission tool
- No processing occurs on our infrastructure
- Your server infrastructure handles all processing

## Transparency Report

As we don't store or have access to user data:

- **Government Requests**: 0 (we have no data to provide)
- **Data Breaches**: 0 (we don't store user data)
- **Third-Party Requests**: 0 (we don't share data)

## Technical Details

### Data Minimization

- Only essential data for functionality is collected
- No decorative or unnecessary data points
- Efficient compression reduces bandwidth usage

### Privacy by Design

- Local-first architecture
- No default cloud services
- User explicitly configures all endpoints
- Opt-in for all data types

### Open Source

The Ariata project is open source, allowing you to:

- Inspect the code for privacy compliance
- Verify data handling practices
- Contribute privacy improvements
- Fork and customize for your needs

---

By using Ariata, you acknowledge that you have read and understood this Privacy Policy and agree to the collection and use of information in accordance with this policy.
