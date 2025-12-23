# Deployment Guide

## DigitalOcean Deployment

### Environment Variables

The application requires the following environment variables to be set in your DigitalOcean App Platform:

#### Required Variables

```bash
APP_ENVIRONMENT=PRODUCTION
APP__database__host=<your-database-host>
APP__database__username=<your-database-username>
APP__database__password=<your-database-password>
APP__database__database_name=<your-database-name>
```

#### Optional Variables

These have defaults from `configuration/base.yaml` but can be overridden:

```bash
APP__database__port=5432
APP__database__max_connections=25
APP__app__port=8000
```

### Setting Environment Variables in DigitalOcean

#### Option 1: Using DigitalOcean Managed Database

If you're using a DigitalOcean Managed PostgreSQL database, you can reference the database connection info:

1. Go to your App in the DigitalOcean dashboard
2. Navigate to **Settings** → **App-Level Environment Variables**
3. Add the following variables:
   - `APP_ENVIRONMENT` = `PRODUCTION`
   - `APP__database__host` = `${db.HOSTNAME}` (if db component is named "db")
   - `APP__database__port` = `${db.PORT}`
   - `APP__database__username` = `${db.USERNAME}`
   - `APP__database__password` = `${db.PASSWORD}`
   - `APP__database__database_name` = `${db.DATABASE}`

#### Option 2: Using External Database (e.g., Supabase)

1. Go to your App in the DigitalOcean dashboard
2. Navigate to **Settings** → **App-Level Environment Variables**
3. Add each variable with the actual values from your database provider
4. Mark `APP__database__password` as **encrypted** (checkbox in the UI)

### Verifying Configuration

After deployment, you can check that the configuration loaded correctly by reviewing the application logs in the DigitalOcean dashboard.

The application will fail to start if required environment variables are missing, with a clear error message indicating which variables are needed.

### Local Development

For local development, use `configuration/local.yaml` (which is gitignored). Example:

```yaml
database:
  host: localhost
  port: 5432
  username: app
  password: secret
  database_name: newsletter
```

Set your local environment:
```bash
export APP_ENVIRONMENT=LOCAL
```
