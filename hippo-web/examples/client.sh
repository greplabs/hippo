#!/bin/bash
# Example client script demonstrating Hippo Web API usage

API_URL="${HIPPO_API_URL:-http://127.0.0.1:3000/api}"

echo "=== Hippo Web API Client Example ==="
echo "API URL: $API_URL"
echo ""

# Health check
echo "1. Health Check"
curl -s "$API_URL/health" | jq .
echo ""

# Get stats
echo "2. Get Statistics"
curl -s "$API_URL/stats" | jq .
echo ""

# Search all
echo "3. Search all memories (limit 5)"
curl -s "$API_URL/search?limit=5" | jq '{total: .total_count, count: (.memories | length)}'
echo ""

# Search with query
echo "4. Search with query 'test'"
curl -s "$API_URL/search?q=test&limit=3" | jq '{total: .total_count, count: (.memories | length)}'
echo ""

# List sources
echo "5. List Sources"
curl -s "$API_URL/sources" | jq .
echo ""

# List tags
echo "6. List Tags (top 10)"
curl -s "$API_URL/tags" | jq 'sort_by(-.count) | .[0:10]'
echo ""

# Add a source (commented out - modify path as needed)
# echo "7. Add Source"
# curl -s -X POST "$API_URL/sources" \
#   -H "Content-Type: application/json" \
#   -d '{"sourceType": "Local", "path": "/Users/you/TestFolder"}' | jq .
# echo ""

# Search by type
echo "7. Search Images only (limit 5)"
curl -s "$API_URL/search?type=Image&limit=5" | jq '{total: .total_count, count: (.memories | length)}'
echo ""

# Search with tags
echo "8. Search with tags"
curl -s "$API_URL/search?tags=important&limit=5" | jq '{total: .total_count, count: (.memories | length)}'
echo ""

# Search with sorting
echo "9. Search sorted by date (newest first)"
curl -s "$API_URL/search?sort=DateNewest&limit=5" | jq '.memories | map({path: .memory.path, date: .memory.indexed_at})'
echo ""

echo "=== Examples Complete ==="
echo ""
echo "To test other endpoints:"
echo "  - Get memory: curl $API_URL/memories/<UUID>"
echo "  - Get thumbnail: curl $API_URL/thumbnails/<UUID> -o thumb.jpg"
echo "  - Add tag: curl -X POST $API_URL/memories/<UUID>/tags -d '{\"tag\": \"test\"}'"
echo "  - Remove tag: curl -X DELETE $API_URL/memories/<UUID>/tags/test"
