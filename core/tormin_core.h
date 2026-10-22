// TorminCore: a small C API over libtorrent for the Windows (WinUI) frontend.
//
// Conventions:
//  - all strings are UTF-8;
//  - returned `const char*` point to thread-local storage and stay valid until
//    the next call into TorminCore on the same thread, so copy them right away;
//  - functions returning int return 0 on success and -1 on failure,
//    the reason is available from tm_last_error();
//  - torrents are identified by ids (> 0) that are never reused within a session.
#pragma once
#include <stdint.h>

#if !defined(_WIN32)
#define TM_API __attribute__((visibility("default")))
#elif defined(TORMIN_CORE_EXPORTS)
#define TM_API __declspec(dllexport)
#else
#define TM_API __declspec(dllimport)
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct tm_session tm_session;
typedef struct tm_tinfo tm_tinfo;

enum tm_state {
    TM_STATE_UNKNOWN = 0,
    TM_STATE_CHECKING = 1,
    TM_STATE_METADATA = 2,
    TM_STATE_DOWNLOADING = 3,
    TM_STATE_FINISHED = 4,
    TM_STATE_SEEDING = 5,
};

enum tm_event_type {
    TM_EVENT_NONE = 0,
    TM_EVENT_METADATA_RECEIVED = 1,
    TM_EVENT_RESUME_DATA = 2,     // data/data_len hold the .fastresume content
    TM_EVENT_RESUME_FAILED = 3,
    TM_EVENT_ERROR = 4,           // message holds the description
    TM_EVENT_FINISHED = 5,
    TM_EVENT_TRACKER_ERROR = 6,  // message holds the description; usually transient
    TM_EVENT_LOG = 7,            // only with TORMIN_LOG_ALERTS set: every libtorrent alert
};

typedef struct tm_status {
    int32_t state;            // tm_state
    int32_t paused;
    int32_t has_metadata;
    float progress;           // 0..1
    int64_t download_rate;    // bytes/s
    int64_t upload_rate;      // bytes/s
    int64_t total_wanted;     // bytes selected for download
    int64_t total_wanted_done;
    int32_t num_peers;
    int32_t num_seeds;
} tm_status;

typedef struct tm_event {
    int32_t type;             // tm_event_type
    uint32_t torrent;         // torrent id, 0 if not related to a torrent
    const char* message;      // valid until the next tm_next_event call
    const uint8_t* data;      // valid until the next tm_next_event call
    int64_t data_len;
} tm_event;

TM_API const char* tm_version(void);
TM_API const char* tm_last_error(void);

// Session
TM_API tm_session* tm_session_create(const char* listen_interfaces);
TM_API void tm_session_destroy(tm_session* s);
// Fetch the next pending event. Returns 1 when `ev` was filled, 0 when the queue is empty.
TM_API int tm_next_event(tm_session* s, tm_event* ev);
// Block up to `timeout_ms` until an event may be available. Returns 1 if so.
TM_API int tm_wait_event(tm_session* s, int timeout_ms);
TM_API int tm_set_rate_limits(tm_session* s, int download_bytes, int upload_bytes);

// Adding torrents. Return the new torrent id, or 0 on failure.
TM_API uint32_t tm_add_magnet(tm_session* s, const char* uri, const char* save_path, int paused);
// `priorities` may be NULL (download everything), otherwise `count` entries 0..7.
TM_API uint32_t tm_add_torrent_file(tm_session* s, const char* path, const char* save_path, int paused,
                                    const uint8_t* priorities, int count);
// `save_path` is used only if the resume data has none.
TM_API uint32_t tm_add_resume_data(tm_session* s, const uint8_t* data, int64_t len, const char* save_path);

// Torrent operations
TM_API int tm_torrent_status(tm_session* s, uint32_t id, tm_status* out);
TM_API const char* tm_torrent_name(tm_session* s, uint32_t id);
TM_API const char* tm_torrent_key(tm_session* s, uint32_t id);   // hex info hash
TM_API const char* tm_torrent_save_path(tm_session* s, uint32_t id);
TM_API int tm_torrent_pause(tm_session* s, uint32_t id);
TM_API int tm_torrent_resume(tm_session* s, uint32_t id);
TM_API int tm_torrent_remove(tm_session* s, uint32_t id, int delete_files);
TM_API int tm_torrent_save_resume(tm_session* s, uint32_t id);
TM_API int tm_torrent_need_save_resume(tm_session* s, uint32_t id);
TM_API int tm_torrent_set_download_limit(tm_session* s, uint32_t id, int bytes_per_sec);
TM_API int tm_torrent_download_limit(tm_session* s, uint32_t id);
// Returns the number of files written (at most `count`), or -1.
TM_API int tm_torrent_get_priorities(tm_session* s, uint32_t id, uint8_t* out, int count);
TM_API int tm_torrent_set_priorities(tm_session* s, uint32_t id, const uint8_t* priorities, int count);
// File list of a torrent with metadata; free with tm_tinfo_free. NULL if no metadata yet.
TM_API tm_tinfo* tm_torrent_info(tm_session* s, uint32_t id);

// Torrent metadata (from a .torrent file or a running torrent)
TM_API tm_tinfo* tm_tinfo_load(const char* path);
TM_API void tm_tinfo_free(tm_tinfo* ti);
TM_API const char* tm_tinfo_name(tm_tinfo* ti);
TM_API int tm_tinfo_num_files(tm_tinfo* ti);
TM_API const char* tm_tinfo_file_path(tm_tinfo* ti, int index);  // '/'-separated, relative
TM_API int64_t tm_tinfo_file_size(tm_tinfo* ti, int index);
TM_API int64_t tm_tinfo_total_size(tm_tinfo* ti);

#ifdef __cplusplus
}
#endif
