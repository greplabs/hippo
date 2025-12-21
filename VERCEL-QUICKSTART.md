# Vercel Deployment - Quick Start

Deploy the Hippo demo to Vercel in 2 minutes.

## Prerequisites

- GitHub account (for GitHub integration)
- OR Vercel CLI installed (`npm install -g vercel`)

## Method 1: One-Click Deploy (Easiest)

Click this button:

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https://github.com/greplabs/hippo)

That's it! Your demo will be live in ~30 seconds.

## Method 2: Vercel CLI

```bash
# Install Vercel CLI
npm install -g vercel

# Login to Vercel
vercel login

# Deploy (from project root)
cd /Users/punitmishra/Downloads/hippov20
vercel

# Follow the prompts:
# - Set up and deploy? Y
# - Which scope? (choose your account)
# - Link to existing project? N
# - What's your project's name? hippo-demo (or your choice)
# - In which directory is your code located? ./
# - Want to override settings? N

# Your demo is now live at: https://hippo-demo-xxxxx.vercel.app
```

### Deploy to Production

```bash
vercel --prod
```

## Method 3: GitHub Integration

1. Push this code to GitHub
2. Go to https://vercel.com/new
3. Import your repository
4. Vercel auto-detects settings from `vercel.json`
5. Click "Deploy"
6. Done! Auto-deploys on every push to main

## What Gets Deployed

The `demo/` directory containing:
- Static HTML/JS/CSS
- Mock API (JSON files)
- Sample data (247 files)
- PWA manifest
- Icons

**Size**: ~20KB (excluding icons)

## Verify Deployment

Test these URLs after deployment:

```bash
# Replace with your deployment URL
DEMO_URL="https://your-project.vercel.app"

# Health check
curl $DEMO_URL/api/health.json

# Stats
curl $DEMO_URL/api/stats.json

# Web UI
open $DEMO_URL  # or visit in browser
```

## Customization

### Update Sample Data

```bash
# Edit mock API files
cd demo/api
nano stats.json
nano search.json

# Commit and push (if using GitHub integration)
git add .
git commit -m "Update sample data"
git push

# Or redeploy manually
vercel --prod
```

### Custom Domain

```bash
# Add domain
vercel domains add yourdomain.com

# Follow DNS instructions in Vercel dashboard
```

## Troubleshooting

### Build Failed

Check `vercel.json` syntax:
```bash
cat vercel.json | jq .
```

### 404 Errors

Ensure you're in the project root:
```bash
pwd  # should show .../hippov20
ls vercel.json  # should exist
```

### API Not Working

Check browser console (F12) for errors. Verify:
```bash
ls demo/api/*.json  # all files should exist
```

## Next Steps

1. ✅ Demo deployed
2. [ ] Test all features
3. [ ] Add custom domain
4. [ ] Enable analytics
5. [ ] Share with users

## Resources

- **Full Guide**: [DEPLOYMENT.md](DEPLOYMENT.md)
- **Setup Summary**: [VERCEL-SETUP-SUMMARY.md](VERCEL-SETUP-SUMMARY.md)
- **Deployment Checklist**: [demo/DEPLOY-CHECKLIST.md](demo/DEPLOY-CHECKLIST.md)
- **Vercel Docs**: https://vercel.com/docs

## Support

Issues? Open a ticket: https://github.com/greplabs/hippo/issues

---

**Deployment Time**: ~2 minutes
**Total Size**: ~20KB (excluding icons)
**Cost**: Free (Vercel Hobby plan)
