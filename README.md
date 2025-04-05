# Platy Paste (backend)

The back end that that is used to store, edit and delete pastes!

You can view a live version of platy paste [here](https://paste.mplaty.com/)

You can interact with the live backend with: `https://paste-api.mplaty.com/`


## Features

Below is a list of features. The checked boxes mean they have been implemented.

- [ ] Rest
    - [ ] Paste
        - [x] Fetch
            - [x] Singular
            - [x] Multiple
        - [ ] Post
            - [x] Singular
            - [ ] Limits (name, type, content)
        - [ ] Patch
            - [ ] Singular
        - [x] Delete
            - [x] Singular
            - [x] Multiple
    - [ ] Document
        - [ ] Fetch
            - [ ] Singular
            - [ ] Multiple
    - [ ] Document Types
        - [ ] Fetch
            - [ ] Singular
- [ ] Authentication
    - [ ] User
    - [ ] Bot


## Setup

This application is currently built and made for docker.

> [!NOTE]
> This project can be ran directly with `cargo run` but requires a [postgres](https://www.postgresql.org/) and [minio](https://min.io/) instance running.

### Step 1

Clone this repository, and open the base directory its in. (where the `/src` folder is.)

### Step 2

Create a .env file (you can copy the contents of the [`.env.example`](https://github.com/mplatypus/platy-paste-backend/blob/main/.env.example))

The `DOMAIN` variable should be set to the place the domain of the frontend.

The `HOST` and `PORT` variables are what the local server will run on, and be accessible by.

The `DATABASE_URL` should be set to the docker network name.

The `MINIO_ROOT_USER` and `MINIO_ROOT_PASSWORD` are required by the end user.

The `S3_ACCESS_KEY` and `S3_SECRET_KEY` will be explained how to get later.

The `S3_URL is where the server is hosted by default. The host should be the docker network name.

### Step 3

Run the following command

```
docker compose up -d
```

> [!TIP]
> On linux, docker usually needs sudo to run.

> [!NOTE]
> The servers should build, but the backend will fail, due to having missing MINIO credentials. The next step(s) will go through how to obtain them.

### Step 4

Log into your minio instance. It will be located at the `http://your-host-here:9001/`, using the credentials `MINIO_ROOT_USER` as the username, and `MINIO_ROOT_PASSWORD` as the password.

### Step 5

under the `User` tab, select the `Access Keys` tab.

On that page, press the `Create access key +` button.

Fill out any information like `expiry` (not recommended) and the `name`/`description`.

Press the `Create` button, and use the created `Access Key` and enter it in your environment file, under the `S3_ACCESS_KEY` and the `Secret Key under the `S3_SECRET_KEY` variable.

Restart your docker containers by running the following command.

```
docker compose restart
```

> [!TIP]
> On linux, docker usually needs sudo to run.


## Important Links

- [Frontend Github](https://github.com/mplatypus/platy-paste-frontend)
- [Documentation Github](https://github.com/mplatypus/platy-paste-documentation)
