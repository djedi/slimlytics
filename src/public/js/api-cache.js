// API Response Cache to prevent duplicate requests
window.APICache = (() => {
	const cache = new Map();
	const CACHE_TTL = 5000; // 5 seconds TTL for cache entries

	function getCacheKey(url, options = {}) {
		return `${url}::${JSON.stringify(options)}`;
	}

	function isExpired(timestamp) {
		return Date.now() - timestamp > CACHE_TTL;
	}

	async function fetchWithCache(url, options = {}) {
		const key = getCacheKey(url, options);
		const cached = cache.get(key);

		// Return cached response if valid
		if (cached && !isExpired(cached.timestamp)) {
			console.log('[APICache] Cache hit for:', url);
			return Promise.resolve(cached.response.clone());
		}

		// Remove expired entry
		if (cached && isExpired(cached.timestamp)) {
			cache.delete(key);
		}

		console.log('[APICache] Cache miss for:', url);
		
		// Make the actual request
		const response = await fetch(url, options);
		
		// Cache successful responses
		if (response.ok) {
			cache.set(key, {
				response: response.clone(),
				timestamp: Date.now()
			});
		}

		return response;
	}

	// Clear cache entries older than TTL
	function cleanup() {
		const now = Date.now();
		for (const [key, value] of cache.entries()) {
			if (isExpired(value.timestamp)) {
				cache.delete(key);
			}
		}
	}

	// Run cleanup every 10 seconds
	setInterval(cleanup, 10000);

	// Clear cache on site change
	window.addEventListener('site-changed', () => {
		console.log('[APICache] Site changed, clearing cache');
		cache.clear();
	});

	return {
		fetch: fetchWithCache,
		clear: () => cache.clear(),
		size: () => cache.size
	};
})();