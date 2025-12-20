#!/usr/bin/env python3
"""
Example Python client for Hippo Web API

Install dependencies:
    pip install requests

Usage:
    python client.py
"""

import os
import requests
from typing import Optional, List, Dict, Any

class HippoClient:
    """Simple Python client for Hippo Web API"""

    def __init__(self, base_url: str = "http://127.0.0.1:3000/api"):
        self.base_url = base_url.rstrip('/')
        self.session = requests.Session()

    def health(self) -> Dict[str, Any]:
        """Check API health"""
        response = self.session.get(f"{self.base_url}/health")
        response.raise_for_status()
        return response.json()

    def stats(self) -> Dict[str, Any]:
        """Get index statistics"""
        response = self.session.get(f"{self.base_url}/stats")
        response.raise_for_status()
        return response.json()

    def search(
        self,
        query: Optional[str] = None,
        tags: Optional[List[str]] = None,
        kind: Optional[str] = None,
        sort: str = "Relevance",
        limit: int = 50,
        offset: int = 0
    ) -> Dict[str, Any]:
        """
        Search memories

        Args:
            query: Search text
            tags: List of tags to filter (prefix with '-' to exclude)
            kind: Memory type (Image, Video, Audio, Code, Document)
            sort: Sort order (Relevance, DateNewest, DateOldest, NameAsc, NameDesc, SizeAsc, SizeDesc)
            limit: Maximum results
            offset: Pagination offset

        Returns:
            Search results with memories, total count, and suggested tags
        """
        params = {
            "limit": limit,
            "offset": offset,
            "sort": sort
        }

        if query:
            params["q"] = query
        if tags:
            params["tags"] = ",".join(tags)
        if kind:
            params["type"] = kind

        response = self.session.get(f"{self.base_url}/search", params=params)
        response.raise_for_status()
        return response.json()

    def get_memory(self, memory_id: str) -> Dict[str, Any]:
        """Get a single memory by ID"""
        response = self.session.get(f"{self.base_url}/memories/{memory_id}")
        response.raise_for_status()
        return response.json()

    def get_thumbnail(self, memory_id: str, output_path: str):
        """Download thumbnail for a memory"""
        response = self.session.get(f"{self.base_url}/thumbnails/{memory_id}")
        response.raise_for_status()

        with open(output_path, 'wb') as f:
            f.write(response.content)

    def list_sources(self) -> List[Dict[str, Any]]:
        """List all sources"""
        response = self.session.get(f"{self.base_url}/sources")
        response.raise_for_status()
        return response.json()

    def add_source(self, path: str, source_type: str = "Local") -> Dict[str, Any]:
        """Add a new source"""
        payload = {
            "sourceType": source_type,
            "path": path
        }
        response = self.session.post(f"{self.base_url}/sources", json=payload)
        response.raise_for_status()
        return response.json()

    def remove_source(self, path: str, delete_files: bool = False) -> Dict[str, Any]:
        """Remove a source"""
        params = {"deleteFiles": "true" if delete_files else "false"}
        response = self.session.delete(f"{self.base_url}/sources/{path}", params=params)
        response.raise_for_status()
        return response.json()

    def list_tags(self) -> List[Dict[str, Any]]:
        """List all tags with counts"""
        response = self.session.get(f"{self.base_url}/tags")
        response.raise_for_status()
        return response.json()

    def add_tag(self, memory_id: str, tag: str) -> Dict[str, Any]:
        """Add a tag to a memory"""
        payload = {"tag": tag}
        response = self.session.post(f"{self.base_url}/memories/{memory_id}/tags", json=payload)
        response.raise_for_status()
        return response.json()

    def remove_tag(self, memory_id: str, tag: str) -> Dict[str, Any]:
        """Remove a tag from a memory"""
        response = self.session.delete(f"{self.base_url}/memories/{memory_id}/tags/{tag}")
        response.raise_for_status()
        return response.json()


def main():
    """Example usage"""
    # Initialize client
    api_url = os.getenv("HIPPO_API_URL", "http://127.0.0.1:3000/api")
    client = HippoClient(api_url)

    print("=== Hippo Web API Python Client Example ===")
    print(f"API URL: {api_url}\n")

    # 1. Health check
    print("1. Health Check")
    health = client.health()
    print(f"   Status: {health['status']}, Version: {health['version']}\n")

    # 2. Get stats
    print("2. Statistics")
    stats = client.stats()
    print(f"   Total memories: {stats['total_memories']}")
    print(f"   By kind: {stats['by_kind']}\n")

    # 3. Search all
    print("3. Search all memories (limit 5)")
    results = client.search(limit=5)
    print(f"   Total: {results['total_count']}, Returned: {len(results['memories'])}\n")

    # 4. Search with query
    print("4. Search with query 'test'")
    results = client.search(query="test", limit=3)
    print(f"   Total: {results['total_count']}, Returned: {len(results['memories'])}\n")

    # 5. List sources
    print("5. List Sources")
    sources = client.list_sources()
    print(f"   Found {len(sources)} sources\n")

    # 6. List tags
    print("6. List Tags (top 10)")
    tags = client.list_tags()
    sorted_tags = sorted(tags, key=lambda x: x['count'], reverse=True)[:10]
    for tag in sorted_tags:
        print(f"   - {tag['name']}: {tag['count']}")
    print()

    # 7. Search by type
    print("7. Search Images only (limit 5)")
    results = client.search(kind="Image", limit=5)
    print(f"   Total images: {results['total_count']}\n")

    # 8. Search with tags
    print("8. Search with tags")
    results = client.search(tags=["important"], limit=5)
    print(f"   Total with 'important' tag: {results['total_count']}\n")

    # 9. Search with sorting
    print("9. Search sorted by date (newest first)")
    results = client.search(sort="DateNewest", limit=5)
    for result in results['memories']:
        memory = result['memory']
        print(f"   - {memory['path']}")
        print(f"     Indexed: {memory['indexed_at']}")
    print()

    print("=== Examples Complete ===\n")
    print("Additional operations:")
    print("  - Get memory: client.get_memory(<UUID>)")
    print("  - Get thumbnail: client.get_thumbnail(<UUID>, 'thumb.jpg')")
    print("  - Add tag: client.add_tag(<UUID>, 'test')")
    print("  - Remove tag: client.remove_tag(<UUID>, 'test')")


if __name__ == "__main__":
    try:
        main()
    except requests.exceptions.ConnectionError:
        print("ERROR: Could not connect to Hippo API")
        print("Make sure the server is running with: cargo run")
    except Exception as e:
        print(f"ERROR: {e}")
