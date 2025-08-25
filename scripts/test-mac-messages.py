#!/usr/bin/env python3
"""
Test script for Mac Messages stream
====================================

This script tests the iMessage stream functionality end-to-end:
1. Creates a test device token
2. Sends mock messages to the API
3. Verifies they're stored correctly in the database
"""

import json
import requests
import psycopg2
from datetime import datetime, timedelta
import random
import string

# Configuration
API_URL = "http://localhost:3000/api/ingest"
DB_CONFIG = {
    "host": "localhost",
    "port": 5432,
    "database": "ariata",
    "user": "ariata_user",
    "password": "ariata_password"
}

def generate_test_token():
    """Generate a random test device token"""
    return ''.join(random.choices(string.ascii_uppercase + string.digits, k=32))

def create_test_device(token):
    """Create a test device source in the database"""
    conn = psycopg2.connect(**DB_CONFIG)
    cur = conn.cursor()
    
    try:
        # Check if source already exists
        cur.execute("""
            SELECT id FROM sources 
            WHERE device_token = %s
        """, (token.upper(),))
        
        existing = cur.fetchone()
        if existing:
            print(f"✓ Device already exists with ID: {existing[0]}")
            return existing[0]
        
        # Create new source
        cur.execute("""
            INSERT INTO sources (
                source_name, instance_name, device_token, 
                auth_type, status, created_at
            ) VALUES (
                'mac', 'Test Mac Device', %s,
                'device_token', 'active', NOW()
            ) RETURNING id
        """, (token.upper(),))
        
        source_id = cur.fetchone()[0]
        conn.commit()
        print(f"✓ Created test device with ID: {source_id}")
        return source_id
        
    finally:
        cur.close()
        conn.close()

def generate_test_messages(count=10):
    """Generate mock message data"""
    messages = []
    base_time = datetime.now() - timedelta(days=7)
    
    for i in range(count):
        # Vary the time across the last 7 days
        message_time = base_time + timedelta(
            days=random.randint(0, 6),
            hours=random.randint(0, 23),
            minutes=random.randint(0, 59)
        )
        
        is_from_me = random.choice([True, False])
        
        message = {
            "message_id": f"TEST-{i:04d}-{random.randint(1000, 9999)}",
            "chat_id": f"chat{random.randint(1, 5):03d}",
            "handle_id": f"+1555{random.randint(1000000, 9999999)}",
            "text": f"Test message {i}: {random.choice(['Hello!', 'How are you?', 'Great!', 'See you later', 'Thanks!'])}",
            "service": random.choice(["iMessage", "SMS"]),
            "is_from_me": is_from_me,
            "date": message_time.isoformat() + "Z",
            "is_read": True if not is_from_me else False,
            "is_delivered": True,
            "is_sent": True if is_from_me else False,
            "cache_has_attachments": random.choice([False, False, False, True]),  # 25% chance
            "attachment_count": random.randint(1, 3) if random.random() < 0.25 else 0
        }
        
        # Add optional fields randomly
        if random.random() < 0.3:
            message["group_title"] = f"Group Chat {random.randint(1, 3)}"
        
        if message["is_read"]:
            message["date_read"] = (message_time + timedelta(minutes=random.randint(1, 60))).isoformat() + "Z"
        
        if message["is_delivered"]:
            message["date_delivered"] = (message_time + timedelta(seconds=random.randint(1, 30))).isoformat() + "Z"
        
        messages.append(message)
    
    return messages

def send_messages_to_api(token, messages):
    """Send messages to the API endpoint"""
    payload = {
        "stream_name": "mac_messages",
        "device_id": "test-device-001",
        "data": messages,
        "batch_metadata": {
            "total_records": len(messages)
        }
    }
    
    headers = {
        "Content-Type": "application/json",
        "X-Device-Token": token
    }
    
    print(f"\n📤 Sending {len(messages)} messages to API...")
    
    response = requests.post(API_URL, json=payload, headers=headers)
    
    if response.status_code == 200:
        print(f"✅ Successfully sent {len(messages)} messages")
        return True
    else:
        print(f"❌ Failed with status {response.status_code}")
        print(f"Response: {response.text}")
        return False

def verify_messages_in_db(message_ids):
    """Verify messages were stored in the database"""
    conn = psycopg2.connect(**DB_CONFIG)
    cur = conn.cursor()
    
    try:
        # Check stream_mac_messages table
        placeholders = ','.join(['%s'] * len(message_ids))
        cur.execute(f"""
            SELECT message_id, chat_id, text, is_from_me, date
            FROM stream_mac_messages
            WHERE message_id IN ({placeholders})
            ORDER BY date DESC
        """, message_ids)
        
        results = cur.fetchall()
        
        print(f"\n📊 Database Verification:")
        print(f"  Expected: {len(message_ids)} messages")
        print(f"  Found: {len(results)} messages")
        
        if len(results) > 0:
            print("\n  Sample messages found:")
            for row in results[:5]:
                print(f"    - {row[0]}: '{row[2][:50]}...' from_me={row[3]}")
        
        # Test deduplication by counting unique message_ids
        cur.execute("""
            SELECT COUNT(DISTINCT message_id), COUNT(*)
            FROM stream_mac_messages
            WHERE message_id LIKE 'TEST-%'
        """)
        
        unique_count, total_count = cur.fetchone()
        print(f"\n  Deduplication check:")
        print(f"    Unique messages: {unique_count}")
        print(f"    Total records: {total_count}")
        
        if unique_count == total_count:
            print("    ✅ No duplicates found")
        else:
            print(f"    ⚠️  Found {total_count - unique_count} duplicates")
        
        return len(results) == len(message_ids)
        
    finally:
        cur.close()
        conn.close()

def test_incremental_sync(token):
    """Test incremental sync by sending messages in batches"""
    print("\n🔄 Testing Incremental Sync...")
    
    # First batch
    batch1 = generate_test_messages(5)
    batch1_ids = [m["message_id"] for m in batch1]
    
    if send_messages_to_api(token, batch1):
        verify_messages_in_db(batch1_ids)
    
    # Second batch with some duplicates
    batch2 = generate_test_messages(5)
    batch2.extend(batch1[:2])  # Add 2 duplicates
    batch2_ids = [m["message_id"] for m in batch2]
    
    print("\n📤 Sending second batch (with 2 duplicates)...")
    if send_messages_to_api(token, batch2):
        # Verify deduplication worked
        all_ids = list(set(batch1_ids + batch2_ids))
        verify_messages_in_db(all_ids)

def cleanup_test_data():
    """Clean up test data from the database"""
    conn = psycopg2.connect(**DB_CONFIG)
    cur = conn.cursor()
    
    try:
        cur.execute("""
            DELETE FROM stream_mac_messages
            WHERE message_id LIKE 'TEST-%'
        """)
        
        deleted = cur.rowcount
        conn.commit()
        print(f"\n🧹 Cleaned up {deleted} test messages")
        
    finally:
        cur.close()
        conn.close()

def main():
    print("🧪 Mac Messages Stream Test")
    print("=" * 40)
    
    # Generate test token
    token = generate_test_token()
    print(f"\n🔑 Test token: {token}")
    
    # Create test device
    source_id = create_test_device(token)
    
    # Generate and send test messages
    messages = generate_test_messages(10)
    message_ids = [m["message_id"] for m in messages]
    
    if send_messages_to_api(token, messages):
        # Verify messages in database
        if verify_messages_in_db(message_ids):
            print("\n✅ Basic test passed!")
        else:
            print("\n❌ Basic test failed - messages not found in database")
    
    # Test incremental sync
    test_incremental_sync(token)
    
    # Cleanup
    print("\n" + "=" * 40)
    response = input("Clean up test data? (y/n): ")
    if response.lower() == 'y':
        cleanup_test_data()
    
    print("\n✨ Test complete!")

if __name__ == "__main__":
    main()