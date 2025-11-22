# Financial Data Integration Plan

USE plaid = "9.0.1" CRATE

## Executive Summary

After comprehensive research comparing MX and Plaid, **Plaid emerges as the optimal choice** for Ariata's financial data ELT system. This document outlines the integration plan using a normalized, source-agnostic finance domain model that allows future integration of additional providers.

### Key Decisions

1. **Provider Choice**: Plaid over MX
   - Free tier with 200 API calls + 100 live accounts
   - Transparent pricing ($500/month production vs MX's $15k+/year)
   - Superior documentation and developer experience
   - Self-service sandbox access

2. **Architecture**: Normalized finance domain with provider abstraction
   - Generic `finance_*` tables for domain concepts
   - Provider-specific metadata in JSONB
   - Source-agnostic interfaces
   - Future-proof for multiple providers

## Provider Comparison

### Plaid vs MX Analysis

| Category | Plaid | MX | Winner |
|----------|-------|-------|---------|
| **Pricing Transparency** | Excellent - public tiers | Poor - must contact sales | Plaid |
| **Free Tier** | Yes - 200 API calls + 100 live Items | No clear free tier | Plaid |
| **Entry Cost** | $0 to start, $500/mo production | ~$15,000/year minimum | Plaid |
| **Institution Coverage** | 12,000+ globally | 13,000+ (mostly US/Canada) | Plaid (global) |
| **Data Quality** | 98% accuracy | Higher quality categorization | MX |
| **Documentation** | Best-in-class | Good, but less extensive | Plaid |
| **Sandbox Access** | Self-service, free | Must request access | Plaid |
| **OAuth Support** | Excellent | Excellent (70% traffic) | Tie |
| **Developer Community** | Large, active | Smaller | Plaid |
| **Rust Support** | Community crate, OpenAPI | REST API only | Plaid |

### Plaid Pricing Details

- **Free Tier**: 200 API calls + 100 live account connections
- **Pay-As-You-Go**: ~$500/month for 1,000 users
- **Per Connection**: $1.50 initial, $0.30/month ongoing
- **Enterprise**: Median $93,000/year with volume discounts

## Architecture Design

### Core Principles

1. **Domain-Driven Design**: Finance concepts independent of providers
2. **Provider Abstraction**: Common interface for all financial sources
3. **Metadata Pattern**: Provider-specific fields in JSONB
4. **Future Extensibility**: Easy addition of MX, Yodlee, direct bank APIs

### Module Structure

```
core/src/sources/
├── finance/                 # Domain logic (source-agnostic)
│   ├── mod.rs
│   ├── models.rs           # FinanceAccount, FinanceTransaction
│   ├── traits.rs           # FinanceProvider trait
│   └── transform.rs        # Common transformations
├── plaid/                  # Plaid-specific implementation
│   ├── mod.rs
│   ├── client.rs          # Plaid API client
│   ├── provider.rs        # Implements FinanceProvider
│   ├── mapper.rs          # Maps Plaid → finance domain
│   ├── sync.rs            # Transaction sync with cursor
│   └── webhook.rs         # JWT verification & handling
```

## Database Schema

### Normalized Finance Domain Tables

```sql
-- Financial institutions
CREATE TABLE finance_institutions (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL,
  routing_number TEXT,
  country_code TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- User connections to institutions
CREATE TABLE finance_connections (
  id SERIAL PRIMARY KEY,
  user_id INTEGER REFERENCES users(id),
  institution_id INTEGER REFERENCES finance_institutions(id),
  connection_status TEXT NOT NULL, -- 'active', 'requires_auth', 'error'
  last_sync_at TIMESTAMPTZ,
  error_code TEXT,
  error_message TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Financial accounts
CREATE TABLE finance_accounts (
  id SERIAL PRIMARY KEY,
  connection_id INTEGER REFERENCES finance_connections(id) ON DELETE CASCADE,
  account_number_masked TEXT,
  account_name TEXT NOT NULL,
  account_type TEXT NOT NULL,    -- 'depository', 'credit', 'loan', 'investment'
  account_subtype TEXT,           -- 'checking', 'savings', 'credit_card'
  currency_code TEXT DEFAULT 'USD',
  current_balance NUMERIC(28,10),
  available_balance NUMERIC(28,10),
  credit_limit NUMERIC(28,10),
  is_active BOOLEAN DEFAULT true,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Transactions (normalized)
CREATE TABLE finance_transactions (
  id SERIAL PRIMARY KEY,
  account_id INTEGER REFERENCES finance_accounts(id) ON DELETE CASCADE,
  external_id TEXT UNIQUE NOT NULL,  -- For deduplication
  amount NUMERIC(28,10) NOT NULL,
  transaction_date DATE NOT NULL,
  posted_date DATE,
  description TEXT NOT NULL,
  merchant_name TEXT,
  category TEXT,
  subcategory TEXT,
  transaction_type TEXT,              -- 'debit', 'credit', 'transfer'
  is_pending BOOLEAN DEFAULT false,
  currency_code TEXT DEFAULT 'USD',
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_finance_transactions_date ON finance_transactions(transaction_date DESC);
CREATE INDEX idx_finance_transactions_account ON finance_transactions(account_id, transaction_date DESC);
CREATE INDEX idx_finance_transactions_external ON finance_transactions(external_id);

-- Provider-specific metadata storage
CREATE TABLE source_metadata (
  id SERIAL PRIMARY KEY,
  source_type TEXT NOT NULL,         -- 'plaid', 'mx', 'yodlee'
  entity_type TEXT NOT NULL,         -- 'connection', 'account', 'transaction'
  entity_id INTEGER NOT NULL,
  metadata JSONB NOT NULL,           -- Provider-specific fields
  created_at TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(source_type, entity_type, entity_id)
);

-- OAuth consent tracking
CREATE TABLE finance_oauth_consents (
  id SERIAL PRIMARY KEY,
  connection_id INTEGER REFERENCES finance_connections(id),
  consent_expires_at TIMESTAMPTZ,
  consent_scope TEXT,
  requires_renewal BOOLEAN DEFAULT false,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Webhook deduplication log
CREATE TABLE finance_webhook_log (
  id SERIAL PRIMARY KEY,
  webhook_id TEXT UNIQUE NOT NULL,
  source_type TEXT NOT NULL,
  webhook_type TEXT NOT NULL,
  connection_id INTEGER REFERENCES finance_connections(id),
  processed_at TIMESTAMPTZ DEFAULT NOW(),
  payload JSONB
);
```

### Metadata Examples

```json
// Connection metadata for Plaid
{
  "plaid_item_id": "M5eVJqLnv3tbzdngLDp9FL5OlDNxlNhlE55op",
  "plaid_access_token": "access-sandbox-...",
  "transactions_cursor": "eyJhY2NvdW50X2lkIjoiQlhSMU..."
}

// Account metadata for Plaid
{
  "plaid_account_id": "BxBXxLj1m4HMXBm9q",
  "plaid_persistent_account_id": "8cfb8beb89b774ee43b090625f0d61d0814322b43bff984eaf60386e"
}

// Transaction metadata for Plaid
{
  "plaid_transaction_id": "lPNjeW1nR6CDn5okmGQ6hEpMo4lLNoSrzqDje",
  "plaid_category": ["Food and Drink", "Restaurants"],
  "plaid_category_id": "13005000",
  "payment_channel": "online"
}
```

## Plaid Integration Best Practices

### Transaction Sync (`/transactions/sync`)

#### Cursor Management (Critical)

```rust
// Initial sync - empty cursor fetches ALL history
let cursor = "";

// Incremental sync - use stored cursor
let cursor = load_cursor(&item_id).await?.unwrap_or_default();

// Forward-only migration from legacy system
let cursor = "now"; // Skip history, start fresh

// CRITICAL: Store cursor ONLY when has_more = false
if !response.has_more {
    store_cursor(&item_id, &response.next_cursor).await?;
}

// NEVER reset cursor on error - restart from original
```

#### Key Considerations

- First sync may take 30-60 seconds (8x normal latency)
- Maximum 500 transactions per API request
- Handle added, modified, AND removed transactions
- Use UPSERT pattern for modified transactions
- Cursor remains valid through access token rotation

### Webhook Architecture

#### JWT Verification (Mandatory)

```rust
// Critical: Body hash uses 2-space JSON formatting
let body_formatted = serde_json::to_string_pretty(&json_value)?;
let hash = sha256(body_formatted);

// Verify:
// 1. Algorithm is ES256
// 2. Signature with Plaid's public key
// 3. Age < 5 minutes
// 4. Body hash matches
```

#### Webhook Types

- `SYNC_UPDATES_AVAILABLE` - New transaction data available
- `PENDING_EXPIRATION` - OAuth consent expiring soon
- `ITEM_LOGIN_REQUIRED` - User must re-authenticate
- `ITEM_ERROR` - Connection error occurred

#### Idempotency Pattern

```rust
// Use inbox pattern with unique constraint
INSERT INTO finance_webhook_log (webhook_id, ...)
ON CONFLICT (webhook_id) DO NOTHING
RETURNING id;

// Process only if insert succeeded
```

### Error Handling

#### Error Categories

1. **User Action Required**
   - `ITEM_LOGIN_REQUIRED`
   - `ACCESS_NOT_GRANTED`
   - Surface to user immediately

2. **Retry with Backoff**
   - `PRODUCT_NOT_READY`
   - `INSTITUTION_DOWN`
   - `RATE_LIMIT_EXCEEDED`
   - Exponential backoff with jitter

3. **Terminal Errors**
   - `PRODUCT_NOT_SUPPORTED`
   - `NO_ACCOUNTS`
   - Log and notify

#### Retry Strategy

```rust
// Exponential backoff with jitter
let delay = initial_delay * (2.0_f64).powi(attempt);
let jitter = rand::gen_range(0.0..=0.5);
let final_delay = delay * (1.0 + jitter);
```

### Rate Limiting

Production limits per endpoint:

- `/transactions/sync`: 30/min per item, 20,000/min total
- `/accounts/balance/get`: 5/min per item
- `/auth/get`: 15/min per item

Implement both per-item AND per-client limits using semaphores.

### OAuth Institution Handling

#### Consent Expiration Tracking

- Store `consent_expires_at` in database
- Monitor when < 30 days remain
- Handle `PENDING_EXPIRATION` webhooks
- Prompt user proactively

#### Key Institutions

- **Chase**: Tokenized account numbers, security questionnaire
- **Capital One**: Yearly consent refresh
- **PNC**: Annual refresh (first expires Oct 2025)
- **Charles Schwab**: 5-week approval period

## Implementation Code Examples

### FinanceProvider Trait

```rust
#[async_trait]
pub trait FinanceProvider: Send + Sync {
    // Connection management
    async fn create_connection(&self, user_id: i32) -> Result<FinanceConnection>;
    async fn refresh_connection(&self, connection_id: i32) -> Result<()>;

    // Data synchronization
    async fn sync_accounts(&self, connection_id: i32) -> Result<Vec<FinanceAccount>>;
    async fn sync_transactions(&self, connection_id: i32) -> Result<TransactionSyncResult>;

    // OAuth management
    async fn get_auth_link(&self, connection_id: i32) -> Result<String>;
    async fn handle_auth_callback(&self, code: &str) -> Result<()>;

    // Webhook handling
    async fn handle_webhook(&self, body: Bytes, headers: HeaderMap) -> Result<()>;
}
```

### Plaid Provider Implementation

```rust
impl FinanceProvider for PlaidProvider {
    async fn sync_transactions(&self, connection_id: i32) -> Result<TransactionSyncResult> {
        // Load Plaid-specific metadata
        let metadata = self.load_source_metadata(connection_id).await?;
        let access_token = metadata.get_str("plaid_access_token")?;
        let cursor = metadata.get_str("transactions_cursor").unwrap_or("");

        // Call Plaid API with cursor
        let response = self.client.transactions_sync(&access_token, cursor, 500).await?;

        // Map to domain models
        let transactions: Vec<FinanceTransaction> = response.added
            .iter()
            .chain(response.modified.iter())
            .map(|txn| self.mapper.map_transaction(txn))
            .collect();

        let removed_ids: Vec<String> = response.removed
            .iter()
            .map(|id| format!("plaid:{}", id))
            .collect();

        // Store new cursor if complete
        if !response.has_more {
            self.update_metadata(connection_id, json!({
                "transactions_cursor": response.next_cursor
            })).await?;
        }

        Ok(TransactionSyncResult {
            transactions,
            removed_ids,
            has_more: response.has_more,
        })
    }
}
```

### Data Mapping

```rust
impl PlaidMapper {
    pub fn map_transaction(&self, plaid_txn: &PlaidTransaction) -> FinanceTransaction {
        FinanceTransaction {
            external_id: format!("plaid:{}", plaid_txn.transaction_id),
            amount: plaid_txn.amount,
            transaction_date: plaid_txn.date,
            description: plaid_txn.name.clone(),
            merchant_name: plaid_txn.merchant_name.clone(),
            category: self.normalize_category(&plaid_txn.category),
            is_pending: plaid_txn.pending,
            transaction_type: if plaid_txn.amount < 0.0 { "debit" } else { "credit" },
            ..Default::default()
        }
    }

    fn normalize_category(&self, plaid_categories: &[String]) -> String {
        // Map Plaid's hierarchical categories to normalized taxonomy
        match plaid_categories.get(0).map(String::as_str) {
            Some("Food and Drink") => "dining",
            Some("Travel") => "travel",
            Some("Shops") => "shopping",
            Some("Transfer") => "transfer",
            Some("Payment") => "payment",
            _ => "other"
        }.to_string()
    }
}
```

## Production Architecture

### Sync Strategy

```rust
pub enum SyncTrigger {
    Webhook,      // Real-time: Process immediately
    Scheduled,    // Batch: Every 6-12 hours as fallback
    Manual,       // On-demand: User-triggered refresh
}

// Hybrid approach:
// 1. Webhooks for real-time updates
// 2. Scheduled sync as fallback for missed webhooks
// 3. Manual refresh on user request
```

### Monitoring Metrics

Key metrics to track:

```rust
pub struct PlaidMetrics {
    // Sync performance
    sync_duration_ms: Histogram,
    sync_success_rate: Counter,
    sync_failure_rate: Counter,

    // API performance
    api_latency_ms: Histogram,
    api_error_rate: Counter,
    rate_limit_hits: Counter,

    // Data quality
    transactions_added: Counter,
    transactions_modified: Counter,
    transactions_removed: Counter,

    // Item health
    items_requiring_auth: Gauge,
    items_in_error: Gauge,
    consent_expiring_soon: Gauge,
}
```

### Alert Thresholds

- **Critical**: `items_requiring_auth > 0` - User action needed
- **Warning**: `oldest_sync_age > 24h` - Potential webhook issues
- **Warning**: `error_rate > 5%` - API health degradation
- **Info**: `consent_expiring_soon > 0` - Proactive notification

## Implementation Roadmap

### Phase 1: Foundation (Week 1)

- [ ] Create normalized finance schema migration
- [ ] Define FinanceProvider trait
- [ ] Set up source_metadata storage pattern
- [ ] Configure Plaid sandbox account

### Phase 2: Plaid Provider (Week 2)

- [ ] Implement PlaidProvider with FinanceProvider trait
- [ ] Add Plaid API client with rate limiting
- [ ] Build data mapping layer
- [ ] Implement transaction sync with cursor management

### Phase 3: Webhook & Auth (Week 3)

- [ ] Source-agnostic webhook router
- [ ] Plaid JWT verification
- [ ] OAuth Link flow integration
- [ ] Comprehensive error handling

### Phase 4: Testing & Polish (Week 4)

- [ ] Sandbox testing suite
- [ ] Limited production testing (100 accounts)
- [ ] Monitoring and alerting setup
- [ ] Documentation and deployment

## Testing Strategy

### Sandbox Testing

```bash
# Use Plaid sandbox credentials
PLAID_CLIENT_ID=sandbox_client_id
PLAID_SECRET=sandbox_secret
PLAID_ENV=sandbox

# Test institutions
- "ins_109508" - First Platypus Bank (OAuth)
- "ins_109509" - First Gingham Credit Union
- "ins_109510" - Tattersall Federal Credit Union

# Simulate webhooks
POST /sandbox/item/fire_webhook
```

### Limited Production

- Test with personal accounts (free 100 Items)
- Validate data accuracy
- Monitor webhook reliability
- Test error scenarios

## Configuration

```json
// config/seeds/finance_providers.json
{
  "finance_providers": {
    "plaid": {
      "enabled": true,
      "client_id": "${PLAID_CLIENT_ID}",
      "secret": "${PLAID_SECRET}",
      "environment": "sandbox",
      "webhook_url": "https://api.ariata.io/webhooks/finance/plaid",
      "products": ["transactions", "accounts", "balances"],
      "country_codes": ["US", "CA"]
    }
  }
}
```

## Security Considerations

1. **Token Storage**: Encrypt access tokens at rest
2. **Webhook Verification**: Always verify JWT signatures
3. **Rate Limiting**: Implement both per-item and per-client limits
4. **Error Handling**: Never expose internal errors to users
5. **Audit Logging**: Log all financial data access
6. **PCI Compliance**: Follow Plaid's security guidelines

## Future Extensibility

### Adding MX Provider

```rust
// Simply implement FinanceProvider trait
impl FinanceProvider for MxProvider {
    // MX-specific implementation
}

// Register with factory
FinanceSourceFactory::register_provider("mx", Box::new(MxProvider::new()));
```

### Direct Bank APIs

- Open Banking (Europe)
- FDX (US)
- Direct institution APIs

### Migration Path

1. Provider-agnostic schema allows switching
2. Metadata pattern preserves provider-specific data
3. External IDs enable re-sync without duplicates

## Common Pitfalls to Avoid

1. **Cursor Management**
   - ❌ Never discard cursor on error
   - ❌ Don't reset cursor on token rotation
   - ✅ Store cursor only when `has_more = false`

2. **Deduplication**
   - ❌ Don't use internal IDs as unique constraint
   - ✅ Use `external_id` with provider prefix

3. **Webhook Processing**
   - ❌ Don't process webhooks synchronously
   - ✅ Use queue with idempotency checks

4. **Error Handling**
   - ❌ Don't retry terminal errors
   - ✅ Distinguish retryable from terminal

5. **Rate Limiting**
   - ❌ Don't use simple counters
   - ✅ Implement per-item AND per-client limits

## References

- [Plaid Documentation](https://plaid.com/docs/)
- [Plaid Pattern Repository](https://github.com/plaid/pattern)
- [Plaid OpenAPI Specification](https://github.com/plaid/plaid-openapi)
- [Plaid Postman Collection](https://www.postman.com/plaid-api/workspace/plaid/overview)
- [MX Documentation](https://docs.mx.com/)

## Appendix: Decision Log

### 2025-11-18: Plaid Selected Over MX

- **Rationale**: Free tier, transparent pricing, superior documentation
- **Trade-offs**: MX has better data categorization
- **Decision**: Start with Plaid, can add MX later if needed

### 2025-11-18: Normalized Schema Over Provider-Specific

- **Rationale**: Future-proof for multiple providers
- **Trade-offs**: Additional mapping complexity
- **Decision**: Use finance_* tables with metadata pattern

---

*This document will be updated as the implementation progresses and new requirements emerge.*
