# bittuly
Distibuted URL Shortener

# Docker commands
```bash
docker compose up -d
```

This starts PostgreSQL on `localhost:5432` with:

- username: `bittu`
- password: `bittu`
- database: `bittuly`
- tables: `users`, `urls`

Postgres init scripts only run when the data volume is first created. If you later change schema and need a clean DB, use:
```bash
  docker compose down -v
  docker compose up -d
```

## Usage
To see the test workflow

Make the bash script runnable
```bash
chmod +x ./scripts/api-test.sh
```
then execute the `api-test.sh` script and it will show the request response cycle for all the requests made.

```bash
./scripts/api-test.sh
```
