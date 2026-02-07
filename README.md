# Platy Paste (backend)

The back end that is used by platy paste to run the site!

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
    - [ ] Paste
        - [ ] Authentication
- [x] Expiry
    - [x] Set expiry
    - [x] Default expiry
    - [x] Expiry task


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

The `OBS_TYPE` will be explained later, and will designate which object storage to use.

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

Based on which type of `OBS_TYPE` was selected, follow the speicifc guide below.

<details>
<summary>Minio</summary>

### Minio
---
Using MINIO, you must set the `OBS_TYPE` to `MINIO` in the environment.

#### Step 1

Log into your minio instance. It will be located at the `http://your-host-here:9001/`, using the credentials `MINIO_ROOT_USER` as the username, and `MINIO_ROOT_PASSWORD` as the password.

#### Step 2

under the `User` tab, select the `Access Keys` tab.

On that page, press the `Create access key +` button.

Fill out any information like `expiry` (not recommended) and the `name`/`description`.

Press the `Create` button, and use the created `Access Key` and enter it in your environment file, under the `OBS_ACCESS_KEY` and the Secret Key under the `OBS_SECRET_KEY` variable.

---
</details>

### Step 5

Restart your docker containers by running the following command.

```
docker compose restart
```

> [!TIP]
> On linux, docker usually needs sudo to run.


## Important Links

- [Frontend Github](https://github.com/mplatypus/platy-paste-frontend)
- [Documentation Github](https://github.com/mplatypus/platy-paste-documentation)
