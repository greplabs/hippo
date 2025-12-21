# Hippo Demo Deployment Checklist

Use this checklist before deploying the demo to production.

## Pre-Deployment

### 1. Content Review

- [ ] Review all sample data in `api/*.json`
- [ ] Ensure no sensitive/private information in mock data
- [ ] Verify file paths in `search.json` are generic (e.g., `/Users/demo/...`)
- [ ] Check that memory counts match across files

### 2. Icons and Assets

- [ ] Add app icons to `icons/` directory
  - [ ] `favicon.ico`
  - [ ] `favicon-16.png`
  - [ ] `favicon-32.png`
  - [ ] `icon-192.png`
  - [ ] `icon-512.png`
  - [ ] `apple-touch-icon.png`
- [ ] Update `manifest.json` with correct paths
- [ ] Test PWA installation locally

### 3. Branding

- [ ] Update page title in `index.html`
- [ ] Update meta descriptions
- [ ] Update Open Graph tags (og:title, og:description, og:image)
- [ ] Update Twitter Card tags
- [ ] Change demo banner message if needed (in `hippo-api-wrapper.js`)

### 4. Configuration

- [ ] Review `vercel.json` settings
- [ ] Check security headers
- [ ] Verify API routes and rewrites
- [ ] Set correct regions if needed

### 5. Testing

- [ ] Test locally with `./serve.sh` or `python3 -m http.server`
- [ ] Test search functionality
- [ ] Test tag filtering
- [ ] Test type filters (Images, Videos, etc.)
- [ ] Test dark mode toggle
- [ ] Test responsive design (mobile, tablet, desktop)
- [ ] Test in different browsers (Chrome, Firefox, Safari, Edge)
- [ ] Verify all links work
- [ ] Check browser console for errors

## Deployment

### Vercel Deployment

- [ ] Install Vercel CLI: `npm install -g vercel`
- [ ] Login to Vercel: `vercel login`
- [ ] Deploy: `vercel` (preview) or `vercel --prod` (production)
- [ ] Note the deployment URL
- [ ] Test the live deployment

### Alternative Platforms

If deploying to Netlify:
- [ ] Create `netlify.toml` (see DEPLOYMENT.md)
- [ ] Deploy via CLI or web interface

If deploying to GitHub Pages:
- [ ] Enable GitHub Pages in repo settings
- [ ] Set source to `main` branch, `/demo` folder

If deploying to Cloudflare Pages:
- [ ] Connect repository
- [ ] Set build output to `demo`

## Post-Deployment

### 1. Verification

- [ ] Visit deployed URL
- [ ] Verify demo banner shows
- [ ] Test search with sample queries
- [ ] Click through different file types
- [ ] Try tag filtering
- [ ] Test dark mode
- [ ] Check mobile responsiveness
- [ ] Verify icons load correctly
- [ ] Test PWA installation

### 2. Performance

- [ ] Run Lighthouse audit
  - [ ] Performance score > 90
  - [ ] Accessibility score > 90
  - [ ] Best Practices score > 90
  - [ ] SEO score > 90
- [ ] Check page load time (should be < 2s)
- [ ] Verify assets are cached properly

### 3. SEO & Social

- [ ] Test Open Graph preview (use https://www.opengraph.xyz/)
- [ ] Test Twitter Card preview
- [ ] Verify meta tags with https://metatags.io/
- [ ] Check mobile-friendliness (Google Mobile-Friendly Test)

### 4. Documentation

- [ ] Update README.md with live demo URL
- [ ] Add deployment date to DEPLOYMENT.md
- [ ] Document any custom configuration
- [ ] Share deployment URL with team

### 5. Monitoring

- [ ] Set up Vercel Analytics (if available)
- [ ] Monitor error logs in Vercel dashboard
- [ ] Check bandwidth usage
- [ ] Review visitor statistics

## Custom Domain Setup

If using a custom domain:

- [ ] Add domain in Vercel dashboard
- [ ] Update DNS records
  - [ ] Add A record or CNAME
  - [ ] Wait for DNS propagation (can take 24-48 hours)
- [ ] Verify SSL certificate is issued
- [ ] Test HTTPS redirect
- [ ] Update all documentation with new URL

## Maintenance

### Weekly

- [ ] Check deployment status
- [ ] Review error logs
- [ ] Monitor bandwidth usage

### Monthly

- [ ] Update sample data if needed
- [ ] Check for broken links
- [ ] Review analytics

### As Needed

- [ ] Update when main app is updated
- [ ] Refresh sample data
- [ ] Update branding/styling

## Rollback Plan

If something goes wrong:

```bash
# Revert to previous deployment in Vercel
vercel rollback

# Or redeploy from a specific commit
git checkout <previous-commit>
vercel --prod
```

## Common Issues

### Icons not loading
- Check file paths in `index.html` and `manifest.json`
- Ensure icon files exist in `icons/` directory
- Verify MIME types are correct

### API calls failing
- Check browser console for CORS errors
- Verify JSON files exist in `api/` directory
- Check `vercel.json` rewrites configuration

### Styling broken
- Clear browser cache
- Check CSS in `index.html`
- Verify no conflicting styles

### PWA not installing
- Ensure HTTPS is enabled
- Check `manifest.json` is valid
- Verify service worker (if added)

## Support

If you encounter issues:

1. Check [DEPLOYMENT.md](../DEPLOYMENT.md)
2. Review [VERCEL-SETUP-SUMMARY.md](../VERCEL-SETUP-SUMMARY.md)
3. Check Vercel documentation
4. Open an issue on GitHub

---

**Last Updated**: 2025-12-21
**Version**: 1.0.0
