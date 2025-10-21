#!/bin/bash

# Test script for report logging functionality
echo "Testing report logging functionality..."

# Sample report content
REPORT_CONTENT="{\"scan_id\":\"test-123\",\"repo\":\"test-repo\",\"findings\":[{\"severity\":\"high\",\"description\":\"Test vulnerability\"}]}"

# Send request to log the report
curl -X POST http://127.0.0.1:8080/api/log-report \
  -H "Content-Type: application/json" \
  -d "{\"report_content\":\"$REPORT_CONTENT\"}"

echo -e "\n\nDone!"