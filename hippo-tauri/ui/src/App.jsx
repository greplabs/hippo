import React, { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

// Icons component
const Icons = {
  hippo: (
    <svg viewBox="0 0 32 32" fill="none" className="w-full h-full">
      <ellipse cx="16" cy="18" rx="12" ry="10" stroke="currentColor" strokeWidth="2"/>
      <ellipse cx="16" cy="14" rx="8" ry="6" stroke="currentColor" strokeWidth="2"/>
      <circle cx="12" cy="12" r="1.5" fill="currentColor"/>
      <circle cx="20" cy="12" r="1.5" fill="currentColor"/>
      <ellipse cx="16" cy="16" rx="3" ry="2" stroke="currentColor" strokeWidth="1.5"/>
      <circle cx="14.5" cy="15.5" r="0.75" fill="currentColor"/>
      <circle cx="17.5" cy="15.5" r="0.75" fill="currentColor"/>
    </svg>
  ),
  search: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-full h-full">
      <circle cx="11" cy="11" r="7"/>
      <path d="M21 21l-4.35-4.35" strokeLinecap="round"/>
    </svg>
  ),
  grid: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-full h-full">
      <rect x="3" y="3" width="7" height="7" rx="1"/>
      <rect x="14" y="3" width="7" height="7" rx="1"/>
      <rect x="3" y="14" width="7" height="7" rx="1"/>
      <rect x="14" y="14" width="7" height="7" rx="1"/>
    </svg>
  ),
  list: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-full h-full">
      <line x1="4" y1="6" x2="20" y2="6"/>
      <line x1="4" y1="12" x2="20" y2="12"/>
      <line x1="4" y1="18" x2="20" y2="18"/>
    </svg>
  ),
  folder: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-full h-full">
      <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>
    </svg>
  ),
  plus: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-full h-full">
      <path d="M12 5v14M5 12h14" strokeLinecap="round"/>
    </svg>
  ),
  close: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-full h-full">
      <path d="M18 6L6 18M6 6l12 12" strokeLinecap="round"/>
    </svg>
  ),
  device: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-full h-full">
      <rect x="2" y="3" width="20" height="14" rx="2"/>
      <line x1="8" y1="21" x2="16" y2="21"/>
      <line x1="12" y1="17" x2="12" y2="21"/>
    </svg>
  ),
  image: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-full h-full">
      <rect x="3" y="3" width="18" height="18" rx="2"/>
      <circle cx="8.5" cy="8.5" r="1.5"/>
      <path d="M21 15l-5-5L5 21"/>
    </svg>
  ),
  code: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-full h-full">
      <path d="M16 18l6-6-6-6M8 6l-6 6 6 6"/>
    </svg>
  ),
  doc: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-full h-full">
      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
      <path d="M14 2v6h6M16 13H8M16 17H8M10 9H8"/>
    </svg>
  ),
  spark: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-full h-full">
      <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
      <circle cx="12" cy="12" r="4"/>
    </svg>
  ),
  refresh: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-full h-full">
      <path d="M23 4v6h-6M1 20v-6h6"/>
      <path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15"/>
    </svg>
  ),
};

function App() {
  const [initialized, setInitialized] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  
  const [searchQuery, setSearchQuery] = useState('');
  const [activeTags, setActiveTags] = useState([]);
  const [viewMode, setViewMode] = useState('grid');
  const [selectedMemory, setSelectedMemory] = useState(null);
  
  const [memories, setMemories] = useState([]);
  const [sources, setSources] = useState([]);
  const [allTags, setAllTags] = useState([]);
  const [stats, setStats] = useState(null);
  
  const searchRef = useRef(null);

  // Initialize Hippo on mount
  useEffect(() => {
    async function init() {
      try {
        setLoading(true);
        const result = await invoke('initialize');
        console.log('Hippo initialized:', result);
        setInitialized(true);
        
        // Load initial data
        await refreshData();
      } catch (err) {
        console.error('Init failed:', err);
        setError(err.toString());
      } finally {
        setLoading(false);
      }
    }
    init();
  }, []);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        searchRef.current?.focus();
      }
      if (e.key === 'Escape') {
        setSelectedMemory(null);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  const refreshData = async () => {
    try {
      const [sourcesData, tagsData, statsData] = await Promise.all([
        invoke('get_sources'),
        invoke('get_tags'),
        invoke('get_stats'),
      ]);
      setSources(sourcesData || []);
      setAllTags(tagsData || []);
      setStats(statsData);
    } catch (err) {
      console.error('Failed to refresh data:', err);
    }
  };

  const handleSearch = async () => {
    try {
      const results = await invoke('search', { 
        query: searchQuery, 
        tags: activeTags 
      });
      setMemories(results.memories || []);
    } catch (err) {
      console.error('Search failed:', err);
    }
  };

  // Search when query or tags change
  useEffect(() => {
    if (initialized) {
      const debounce = setTimeout(handleSearch, 300);
      return () => clearTimeout(debounce);
    }
  }, [searchQuery, activeTags, initialized]);

  const handleAddSource = async () => {
    try {
      // Use Tauri dialog to pick a folder
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select folder to index',
      });
      
      if (selected) {
        setLoading(true);
        await invoke('add_source', { 
          sourceType: 'local', 
          path: selected 
        });
        await refreshData();
        setLoading(false);
      }
    } catch (err) {
      console.error('Failed to add source:', err);
      setError(err.toString());
      setLoading(false);
    }
  };

  const handleTagAdd = (tag) => {
    if (!activeTags.includes(tag)) {
      setActiveTags([...activeTags, tag]);
      setSearchQuery('');
    }
  };

  const handleTagRemove = (tag) => {
    setActiveTags(activeTags.filter(t => t !== tag));
  };

  const handleSearchKeyDown = (e) => {
    if (e.key === 'Tab' && searchQuery.trim()) {
      e.preventDefault();
      handleTagAdd(searchQuery.trim().toLowerCase());
    }
    if (e.key === 'Backspace' && !searchQuery && activeTags.length > 0) {
      handleTagRemove(activeTags[activeTags.length - 1]);
    }
    if (e.key === 'Enter') {
      handleSearch();
    }
  };

  const getTypeIcon = (kind) => {
    if (!kind) return Icons.doc;
    if (kind.Image) return Icons.image;
    if (kind.Code) return Icons.code;
    if (kind.Document) return Icons.doc;
    return Icons.folder;
  };

  const getKindColor = (kind) => {
    if (!kind) return '#E0E0E0';
    if (kind.Image) return '#E8DDD4';
    if (kind.Code) return '#DDD4E8';
    if (kind.Document) return '#D4DDE8';
    return '#E0E0E0';
  };

  // Loading screen
  if (loading && !initialized) {
    return (
      <div className="h-screen w-full bg-stone-50 flex items-center justify-center">
        <div className="text-center">
          <div className="w-16 h-16 mx-auto mb-4 text-stone-600 animate-pulse">
            {Icons.hippo}
          </div>
          <h1 className="text-xl font-semibold text-stone-800 mb-2">Hippo</h1>
          <p className="text-stone-500">Initializing...</p>
        </div>
      </div>
    );
  }

  // Error screen
  if (error && !initialized) {
    return (
      <div className="h-screen w-full bg-stone-50 flex items-center justify-center">
        <div className="text-center max-w-md">
          <div className="w-16 h-16 mx-auto mb-4 text-red-500">
            {Icons.hippo}
          </div>
          <h1 className="text-xl font-semibold text-stone-800 mb-2">Initialization Error</h1>
          <p className="text-red-600 text-sm mb-4">{error}</p>
          <button 
            onClick={() => window.location.reload()}
            className="px-4 py-2 bg-stone-800 text-white rounded-lg"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen w-full bg-stone-50 flex overflow-hidden">
      {/* Sidebar */}
      <aside className="w-56 bg-white border-r border-stone-200 flex flex-col">
        {/* Logo */}
        <div className="p-4 border-b border-stone-100">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 text-stone-700">{Icons.hippo}</div>
            <div>
              <h1 className="text-stone-800 font-semibold text-sm">Hippo</h1>
              <p className="text-stone-400 text-xs">Never forgets</p>
            </div>
          </div>
        </div>

        {/* Sources */}
        <div className="flex-1 overflow-y-auto p-3">
          <div className="flex items-center justify-between px-2 mb-2">
            <p className="text-xs font-semibold text-stone-400 uppercase tracking-wider">Sources</p>
            <button 
              onClick={refreshData}
              className="w-4 h-4 text-stone-400 hover:text-stone-600"
              title="Refresh"
            >
              {Icons.refresh}
            </button>
          </div>
          
          {sources.length === 0 ? (
            <p className="px-2 py-4 text-sm text-stone-400 text-center">
              No sources added yet
            </p>
          ) : (
            sources.map((source, i) => (
              <div
                key={i}
                className="flex items-center gap-2.5 px-2 py-2 rounded-lg hover:bg-stone-50 cursor-pointer group"
              >
                <div className="w-5 h-5 text-stone-500">{Icons.device}</div>
                <span className="flex-1 text-sm text-stone-600 truncate">
                  {source.source?.Local?.root_path?.split('/').pop() || 'Local'}
                </span>
                <div className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
              </div>
            ))
          )}
          
          <button 
            onClick={handleAddSource}
            className="w-full flex items-center gap-2 px-2 py-2 mt-2 rounded-lg text-stone-400 hover:bg-stone-50 hover:text-stone-600 border border-dashed border-stone-200"
          >
            <div className="w-4 h-4">{Icons.plus}</div>
            <span className="text-sm">Add Folder</span>
          </button>
        </div>

        {/* Stats */}
        <div className="p-4 border-t border-stone-100">
          <p className="text-xs text-stone-400 flex items-center gap-1">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
            {stats?.total_memories || 0} memories indexed
          </p>
        </div>
      </aside>

      {/* Main content */}
      <main className="flex-1 flex flex-col overflow-hidden">
        {/* Search bar */}
        <div className="bg-white border-b border-stone-200 p-4">
          <div className="max-w-3xl mx-auto">
            <div className="flex items-center gap-2 bg-stone-50 rounded-xl px-4 py-3 border border-stone-200 focus-within:border-stone-400 focus-within:ring-2 focus-within:ring-stone-100 transition-all">
              <div className="w-5 h-5 text-stone-400">{Icons.search}</div>
              
              {activeTags.map((tag) => (
                <span
                  key={tag}
                  className="flex items-center gap-1 px-2 py-1 bg-stone-200 text-stone-700 rounded-md text-sm"
                >
                  {tag}
                  <button
                    onClick={() => handleTagRemove(tag)}
                    className="w-3.5 h-3.5 text-stone-500 hover:text-stone-700"
                  >
                    {Icons.close}
                  </button>
                </span>
              ))}
              
              <input
                ref={searchRef}
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyDown={handleSearchKeyDown}
                placeholder={activeTags.length ? "Add more filters..." : "Search your memories... (Tab to add as tag)"}
                className="flex-1 bg-transparent outline-none text-stone-800 placeholder-stone-400"
              />
              
              <kbd className="px-2 py-0.5 bg-white border border-stone-200 rounded text-xs text-stone-400">⌘K</kbd>
            </div>

            {/* Tag suggestions */}
            {allTags.length > 0 && (
              <div className="flex items-center gap-2 mt-3 flex-wrap">
                <span className="text-xs text-stone-400">Tags:</span>
                {allTags.slice(0, 10).map(([tag, count]) => (
                  <button
                    key={tag}
                    onClick={() => handleTagAdd(tag)}
                    className="px-2 py-1 bg-stone-100 hover:bg-stone-200 text-stone-600 rounded-md text-xs transition-colors"
                  >
                    {tag} <span className="text-stone-400">({count})</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Toolbar */}
        <div className="bg-white border-b border-stone-100 px-4 py-2 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className="text-sm text-stone-500">{memories.length} memories</span>
          </div>
          
          <div className="flex items-center gap-1 bg-stone-100 rounded-lg p-0.5">
            {[
              { id: 'grid', icon: Icons.grid },
              { id: 'list', icon: Icons.list },
            ].map((view) => (
              <button
                key={view.id}
                onClick={() => setViewMode(view.id)}
                className={`w-8 h-8 rounded-md flex items-center justify-center transition-all ${
                  viewMode === view.id
                    ? 'bg-white text-stone-800 shadow-sm'
                    : 'text-stone-400 hover:text-stone-600'
                }`}
              >
                <div className="w-4 h-4">{view.icon}</div>
              </button>
            ))}
          </div>
        </div>

        {/* Content area */}
        <div className="flex-1 overflow-y-auto p-6">
          {memories.length === 0 ? (
            <div className="h-full flex items-center justify-center">
              <div className="text-center max-w-md">
                <div className="w-16 h-16 mx-auto mb-4 text-stone-300">
                  {Icons.folder}
                </div>
                <h3 className="text-stone-600 font-medium mb-2">
                  {sources.length === 0 ? 'Add a folder to get started' : 'No memories found'}
                </h3>
                <p className="text-stone-400 text-sm">
                  {sources.length === 0 
                    ? 'Click "Add Folder" in the sidebar to index your first folder.'
                    : 'Try adjusting your search or filters.'}
                </p>
              </div>
            </div>
          ) : viewMode === 'grid' ? (
            <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
              {memories.map((result) => (
                <article
                  key={result.memory.id}
                  onClick={() => setSelectedMemory(result.memory)}
                  className="group relative rounded-xl overflow-hidden cursor-pointer transition-all hover:shadow-lg"
                >
                  <div
                    className="aspect-square flex items-center justify-center transition-transform group-hover:scale-105"
                    style={{ backgroundColor: getKindColor(result.memory.kind) }}
                  >
                    <div className="w-8 h-8 text-stone-500/50">
                      {getTypeIcon(result.memory.kind)}
                    </div>
                  </div>
                  
                  <div className="absolute inset-0 bg-gradient-to-t from-black/50 to-transparent opacity-0 group-hover:opacity-100 transition-opacity" />
                  
                  <div className="absolute bottom-0 left-0 right-0 p-3 translate-y-full group-hover:translate-y-0 transition-transform">
                    <p className="text-white font-medium text-sm truncate">
                      {result.memory.metadata?.title || result.memory.path.split('/').pop()}
                    </p>
                  </div>
                </article>
              ))}
            </div>
          ) : (
            <div className="max-w-3xl space-y-1">
              {memories.map((result) => (
                <article
                  key={result.memory.id}
                  onClick={() => setSelectedMemory(result.memory)}
                  className="flex items-center gap-4 p-3 rounded-xl hover:bg-white cursor-pointer transition-colors"
                >
                  <div
                    className="w-12 h-12 rounded-lg flex items-center justify-center"
                    style={{ backgroundColor: getKindColor(result.memory.kind) }}
                  >
                    <div className="w-5 h-5 text-stone-500/70">
                      {getTypeIcon(result.memory.kind)}
                    </div>
                  </div>
                  <div className="flex-1 min-w-0">
                    <h3 className="text-stone-800 font-medium truncate">
                      {result.memory.metadata?.title || result.memory.path.split('/').pop()}
                    </h3>
                    <p className="text-stone-400 text-sm truncate">{result.memory.path}</p>
                  </div>
                  <div className="flex gap-1">
                    {result.memory.tags?.slice(0, 3).map((tag) => (
                      <span key={tag.name} className="px-2 py-0.5 bg-stone-100 text-stone-500 rounded text-xs">
                        {tag.name}
                      </span>
                    ))}
                  </div>
                </article>
              ))}
            </div>
          )}
        </div>
      </main>

      {/* Detail panel */}
      {selectedMemory && (
        <aside className="w-80 bg-white border-l border-stone-200 flex flex-col">
          <div className="flex items-center justify-between p-4 border-b border-stone-100">
            <h2 className="font-medium text-stone-700">Details</h2>
            <button
              onClick={() => setSelectedMemory(null)}
              className="w-6 h-6 text-stone-400 hover:text-stone-600"
            >
              {Icons.close}
            </button>
          </div>

          <div
            className="h-48 flex items-center justify-center"
            style={{ backgroundColor: getKindColor(selectedMemory.kind) }}
          >
            <div className="w-12 h-12 text-stone-500/50">
              {getTypeIcon(selectedMemory.kind)}
            </div>
          </div>

          <div className="flex-1 overflow-y-auto p-4">
            <h1 className="text-lg font-medium text-stone-800 mb-1">
              {selectedMemory.metadata?.title || selectedMemory.path.split('/').pop()}
            </h1>
            <p className="text-stone-400 text-sm mb-4 break-all">{selectedMemory.path}</p>

            {/* Tags */}
            {selectedMemory.tags?.length > 0 && (
              <div className="mb-4">
                <p className="text-xs font-semibold text-stone-400 uppercase tracking-wider mb-2">Tags</p>
                <div className="flex flex-wrap gap-1.5">
                  {selectedMemory.tags.map((tag) => (
                    <span
                      key={tag.name}
                      className="px-2.5 py-1 bg-stone-100 text-stone-600 rounded-full text-sm cursor-pointer hover:bg-stone-200"
                      onClick={() => handleTagAdd(tag.name)}
                    >
                      {tag.name}
                    </span>
                  ))}
                </div>
              </div>
            )}

            {/* Metadata */}
            <div>
              <p className="text-xs font-semibold text-stone-400 uppercase tracking-wider mb-2">Info</p>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-stone-400">Size</span>
                  <span className="text-stone-600">
                    {formatBytes(selectedMemory.metadata?.file_size || 0)}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-stone-400">Type</span>
                  <span className="text-stone-600">
                    {selectedMemory.metadata?.mime_type || 'Unknown'}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </aside>
      )}
    </div>
  );
}

function formatBytes(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

export default App;
