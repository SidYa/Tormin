#define TORMIN_CORE_EXPORTS
#include "tormin_core.h"

#include <libtorrent/alert_types.hpp>
#include <libtorrent/load_torrent.hpp>
#include <libtorrent/magnet_uri.hpp>
#include <libtorrent/read_resume_data.hpp>
#include <libtorrent/session.hpp>
#include <libtorrent/torrent_info.hpp>
#include <libtorrent/version.hpp>
#include <libtorrent/write_resume_data.hpp>

#include <algorithm>
#include <cstdlib>
#include <deque>
#include <memory>
#include <mutex>
#include <sstream>
#include <string>
#include <unordered_map>
#include <vector>

namespace {

thread_local std::string g_error;
thread_local std::string g_str;  // backing storage for returned strings

const char* ret_str(std::string s) {
    g_str = std::move(s);
    return g_str.c_str();
}

int fail(std::string msg) {
    g_error = std::move(msg);
    return -1;
}

struct pending_event {
    int32_t type;
    uint32_t torrent;
    std::string message;
    std::vector<char> data;
};

// Reads a .torrent file (libtorrent 2.1 no longer has the error_code constructor)
bool load_torrent(const char* path, lt::add_torrent_params& out) {
    try {
        out = lt::load_torrent_file(path);
        return true;
    } catch (std::exception const& e) {
        g_error = e.what();
        return false;
    }
}

// File list of a torrent: layout() since libtorrent 2.1, files() before (distributions ship 2.0.x)
lt::file_storage const& file_list(lt::torrent_info const& ti) {
#if LIBTORRENT_VERSION_NUM >= 20100
    return ti.layout();
#else
    return ti.files();
#endif
}

std::string hash_hex(lt::torrent_handle const& h) {
    std::ostringstream os;
    os << h.info_hashes().get_best();
    return os.str();
}

}  // namespace

struct tm_session {
    std::unique_ptr<lt::session> ses;
    std::mutex mtx;
    uint32_t next_id = 1;
    std::unordered_map<uint32_t, lt::torrent_handle> by_id;
    std::unordered_map<lt::torrent_handle, uint32_t> ids;
    std::deque<pending_event> events;
    pending_event current;  // keeps the last returned event's buffers alive
    bool log_alerts = false;  // TORMIN_LOG_ALERTS: forward every alert as TM_EVENT_LOG

    uint32_t track(lt::torrent_handle const& h) {
        auto it = ids.find(h);
        if (it != ids.end()) return it->second;
        uint32_t id = next_id++;
        by_id[id] = h;
        ids[h] = id;
        return id;
    }

    uint32_t id_of(lt::torrent_handle const& h) const {
        auto it = ids.find(h);
        return it == ids.end() ? 0 : it->second;
    }

    lt::torrent_handle* find(uint32_t id) {
        auto it = by_id.find(id);
        if (it == by_id.end() || !it->second.is_valid()) {
            g_error = "unknown torrent id";
            return nullptr;
        }
        return &it->second;
    }

    void forget(uint32_t id) {
        auto it = by_id.find(id);
        if (it == by_id.end()) return;
        ids.erase(it->second);
        by_id.erase(it);
    }

    uint32_t add(lt::add_torrent_params p, bool paused) {
        // Torrents are not auto-managed: pause/resume is fully under user control
        // and every torrent runs at once, like the Linux version.
        // libtorrent's default flags contain paused + auto_managed (the queue unpauses it),
        // so without auto_managed the paused flag must be set explicitly either way.
        p.flags &= ~lt::torrent_flags::auto_managed;
        if (paused)
            p.flags |= lt::torrent_flags::paused;
        else
            p.flags &= ~lt::torrent_flags::paused;
        lt::error_code ec;
        lt::torrent_handle h = ses->add_torrent(std::move(p), ec);
        if (ec) {
            g_error = ec.message();
            return 0;
        }
        return track(h);
    }

    void drain_alerts() {
        std::vector<lt::alert*> alerts;
        ses->pop_alerts(&alerts);
        for (lt::alert* a : alerts) {
            if (log_alerts) events.push_back({TM_EVENT_LOG, 0, a->what() + std::string(": ") + a->message(), {}});
            if (auto* rd = lt::alert_cast<lt::save_resume_data_alert>(a)) {
                auto buf = lt::write_resume_data_buf(rd->params);
                events.push_back({TM_EVENT_RESUME_DATA, id_of(rd->handle), {}, std::move(buf)});
            } else if (auto* rf = lt::alert_cast<lt::save_resume_data_failed_alert>(a)) {
                events.push_back({TM_EVENT_RESUME_FAILED, id_of(rf->handle), rf->message(), {}});
            } else if (auto* md = lt::alert_cast<lt::metadata_received_alert>(a)) {
                events.push_back({TM_EVENT_METADATA_RECEIVED, id_of(md->handle), {}, {}});
            } else if (auto* fin = lt::alert_cast<lt::torrent_finished_alert>(a)) {
                events.push_back({TM_EVENT_FINISHED, id_of(fin->handle), {}, {}});
            } else if (auto* te = lt::alert_cast<lt::torrent_error_alert>(a)) {
                events.push_back({TM_EVENT_ERROR, id_of(te->handle), te->message(), {}});
            } else if (auto* fe = lt::alert_cast<lt::file_error_alert>(a)) {
                events.push_back({TM_EVENT_ERROR, id_of(fe->handle), fe->message(), {}});
            } else if (auto* tr = lt::alert_cast<lt::tracker_error_alert>(a)) {
                events.push_back({TM_EVENT_TRACKER_ERROR, id_of(tr->handle), tr->message(), {}});
            } else if (auto* le = lt::alert_cast<lt::listen_failed_alert>(a)) {
                events.push_back({TM_EVENT_ERROR, 0, le->message(), {}});
            }
        }
    }
};

struct tm_tinfo {
    std::shared_ptr<const lt::torrent_info> ti;
};

#define TM_GUARD(s) \
    if (!(s)) return fail("null session"); \
    std::lock_guard<std::mutex> lock((s)->mtx)

#define TM_HANDLE(s, id, h) \
    lt::torrent_handle* h = (s)->find(id); \
    if (!h) return -1

extern "C" {

const char* tm_version(void) { return LIBTORRENT_VERSION; }

const char* tm_last_error(void) { return g_error.c_str(); }

tm_session* tm_session_create(const char* listen_interfaces) {
    try {
        lt::settings_pack sp;
        sp.set_str(lt::settings_pack::listen_interfaces,
                   listen_interfaces && *listen_interfaces ? listen_interfaces : "0.0.0.0:6881,[::]:6881");
        bool log_alerts = std::getenv("TORMIN_LOG_ALERTS") != nullptr;
        sp.set_int(lt::settings_pack::alert_mask,
                   lt::alert_category::status | lt::alert_category::storage | lt::alert_category::error |
                       lt::alert_category::tracker);
        if (log_alerts) sp.set_int(lt::settings_pack::alert_mask, lt::alert_category::all);
        sp.set_int(lt::settings_pack::auto_scrape_interval, 1800);
        sp.set_int(lt::settings_pack::dht_upload_rate_limit, 4000);
        sp.set_str(lt::settings_pack::user_agent, std::string("Tormin/0.1 libtorrent/") + LIBTORRENT_VERSION);
        auto s = std::make_unique<tm_session>();
        s->ses = std::make_unique<lt::session>(lt::session_params(std::move(sp)));
        s->log_alerts = log_alerts;
        return s.release();
    } catch (std::exception const& e) {
        g_error = e.what();
        return nullptr;
    }
}

void tm_session_destroy(tm_session* s) {
    delete s;  // lt::session's destructor stops the network thread
}

int tm_next_event(tm_session* s, tm_event* ev) {
    if (!s || !ev) return 0;
    std::lock_guard<std::mutex> lock(s->mtx);
    if (s->events.empty()) s->drain_alerts();
    if (s->events.empty()) {
        *ev = {};
        return 0;
    }
    s->current = std::move(s->events.front());
    s->events.pop_front();
    ev->type = s->current.type;
    ev->torrent = s->current.torrent;
    ev->message = s->current.message.c_str();
    ev->data = reinterpret_cast<const uint8_t*>(s->current.data.data());
    ev->data_len = static_cast<int64_t>(s->current.data.size());
    return 1;
}

int tm_wait_event(tm_session* s, int timeout_ms) {
    if (!s) return 0;
    {
        std::lock_guard<std::mutex> lock(s->mtx);
        if (!s->events.empty()) return 1;
    }
    return s->ses->wait_for_alert(lt::milliseconds(timeout_ms)) ? 1 : 0;
}

int tm_set_rate_limits(tm_session* s, int download_bytes, int upload_bytes) {
    TM_GUARD(s);
    lt::settings_pack sp;
    sp.set_int(lt::settings_pack::download_rate_limit, std::max(0, download_bytes));
    sp.set_int(lt::settings_pack::upload_rate_limit, std::max(0, upload_bytes));
    s->ses->apply_settings(std::move(sp));
    return 0;
}

uint32_t tm_add_magnet(tm_session* s, const char* uri, const char* save_path, int paused) {
    if (!s || !uri) return 0;
    std::lock_guard<std::mutex> lock(s->mtx);
    lt::error_code ec;
    lt::add_torrent_params p = lt::parse_magnet_uri(uri, ec);
    if (ec) {
        g_error = ec.message();
        return 0;
    }
    p.save_path = save_path ? save_path : "";
    return s->add(std::move(p), paused != 0);
}

uint32_t tm_add_torrent_file(tm_session* s, const char* path, const char* save_path, int paused,
                             const uint8_t* priorities, int count) {
    if (!s || !path) return 0;
    std::lock_guard<std::mutex> lock(s->mtx);
    lt::add_torrent_params p;
    if (!load_torrent(path, p)) return 0;
    p.save_path = save_path ? save_path : "";
    if (priorities && count > 0) {
        p.file_priorities.reserve(count);
        for (int i = 0; i < count; ++i) p.file_priorities.push_back(lt::download_priority_t(priorities[i]));
    }
    return s->add(std::move(p), paused != 0);
}

uint32_t tm_add_resume_data(tm_session* s, const uint8_t* data, int64_t len, const char* save_path) {
    if (!s || !data || len <= 0) return 0;
    std::lock_guard<std::mutex> lock(s->mtx);
    lt::error_code ec;
    lt::add_torrent_params p = lt::read_resume_data(
        lt::span<char const>(reinterpret_cast<const char*>(data), static_cast<std::ptrdiff_t>(len)), ec);
    if (ec) {
        g_error = ec.message();
        return 0;
    }
    if (p.save_path.empty() && save_path) p.save_path = save_path;
    bool paused = bool(p.flags & lt::torrent_flags::paused);
    return s->add(std::move(p), paused);
}

int tm_torrent_status(tm_session* s, uint32_t id, tm_status* out) {
    TM_GUARD(s);
    TM_HANDLE(s, id, h);
    if (!out) return fail("null output");
    lt::torrent_status st = h->status();
    *out = {};
    switch (st.state) {
        case lt::torrent_status::checking_files:
        case lt::torrent_status::checking_resume_data: out->state = TM_STATE_CHECKING; break;
        case lt::torrent_status::downloading_metadata: out->state = TM_STATE_METADATA; break;
        case lt::torrent_status::downloading: out->state = TM_STATE_DOWNLOADING; break;
        case lt::torrent_status::finished: out->state = TM_STATE_FINISHED; break;
        case lt::torrent_status::seeding: out->state = TM_STATE_SEEDING; break;
        default: out->state = TM_STATE_UNKNOWN; break;
    }
    out->paused = bool(st.flags & lt::torrent_flags::paused);
    out->has_metadata = st.has_metadata;
    out->progress = st.progress;
    out->download_rate = st.download_payload_rate;
    out->upload_rate = st.upload_payload_rate;
    out->total_wanted = st.total_wanted;
    out->total_wanted_done = st.total_wanted_done;
    out->num_peers = st.num_peers;
    out->num_seeds = st.num_seeds;
    return 0;
}

const char* tm_torrent_name(tm_session* s, uint32_t id) {
    if (!s) return nullptr;
    std::lock_guard<std::mutex> lock(s->mtx);
    lt::torrent_handle* h = s->find(id);
    if (!h) return nullptr;
    return ret_str(h->status(lt::torrent_handle::query_name).name);
}

const char* tm_torrent_key(tm_session* s, uint32_t id) {
    if (!s) return nullptr;
    std::lock_guard<std::mutex> lock(s->mtx);
    lt::torrent_handle* h = s->find(id);
    if (!h) return nullptr;
    return ret_str(hash_hex(*h));
}

const char* tm_torrent_save_path(tm_session* s, uint32_t id) {
    if (!s) return nullptr;
    std::lock_guard<std::mutex> lock(s->mtx);
    lt::torrent_handle* h = s->find(id);
    if (!h) return nullptr;
    return ret_str(h->status(lt::torrent_handle::query_save_path).save_path);
}

int tm_torrent_pause(tm_session* s, uint32_t id) {
    TM_GUARD(s);
    TM_HANDLE(s, id, h);
    h->pause(lt::torrent_handle::graceful_pause);
    return 0;
}

int tm_torrent_resume(tm_session* s, uint32_t id) {
    TM_GUARD(s);
    TM_HANDLE(s, id, h);
    h->resume();
    return 0;
}

int tm_torrent_remove(tm_session* s, uint32_t id, int delete_files) {
    TM_GUARD(s);
    TM_HANDLE(s, id, h);
    s->ses->remove_torrent(*h, delete_files ? lt::session::delete_files : lt::remove_flags_t{});
    s->forget(id);
    return 0;
}

int tm_torrent_save_resume(tm_session* s, uint32_t id) {
    TM_GUARD(s);
    TM_HANDLE(s, id, h);
    h->save_resume_data(lt::torrent_handle::save_info_dict);
    return 0;
}

int tm_torrent_need_save_resume(tm_session* s, uint32_t id) {
    TM_GUARD(s);
    TM_HANDLE(s, id, h);
    return h->need_save_resume_data() ? 1 : 0;
}

int tm_torrent_set_download_limit(tm_session* s, uint32_t id, int bytes_per_sec) {
    TM_GUARD(s);
    TM_HANDLE(s, id, h);
    h->set_download_limit(bytes_per_sec > 0 ? bytes_per_sec : -1);
    return 0;
}

int tm_torrent_download_limit(tm_session* s, uint32_t id) {
    TM_GUARD(s);
    TM_HANDLE(s, id, h);
    int limit = h->download_limit();
    return limit > 0 ? limit : 0;
}

int tm_torrent_get_priorities(tm_session* s, uint32_t id, uint8_t* out, int count) {
    TM_GUARD(s);
    TM_HANDLE(s, id, h);
    auto prio = h->get_file_priorities();
    int n = std::min<int>(count, static_cast<int>(prio.size()));
    for (int i = 0; i < n; ++i) out[i] = static_cast<uint8_t>(static_cast<std::uint8_t>(prio[i]));
    return n;
}

int tm_torrent_set_priorities(tm_session* s, uint32_t id, const uint8_t* priorities, int count) {
    TM_GUARD(s);
    TM_HANDLE(s, id, h);
    std::vector<lt::download_priority_t> prio;
    prio.reserve(count);
    for (int i = 0; i < count; ++i) prio.push_back(lt::download_priority_t(priorities[i]));
    h->prioritize_files(prio);
    return 0;
}

tm_tinfo* tm_torrent_info(tm_session* s, uint32_t id) {
    if (!s) return nullptr;
    std::lock_guard<std::mutex> lock(s->mtx);
    lt::torrent_handle* h = s->find(id);
    if (!h) return nullptr;
    auto ti = h->torrent_file();
    if (!ti) {
        g_error = "no metadata yet";
        return nullptr;
    }
    return new tm_tinfo{std::move(ti)};
}

tm_tinfo* tm_tinfo_load(const char* path) {
    if (!path) return nullptr;
    lt::add_torrent_params p;
    if (!load_torrent(path, p)) return nullptr;
    return new tm_tinfo{std::move(p.ti)};
}

void tm_tinfo_free(tm_tinfo* ti) { delete ti; }

const char* tm_tinfo_name(tm_tinfo* ti) { return ti ? ret_str(ti->ti->name()) : nullptr; }

int tm_tinfo_num_files(tm_tinfo* ti) { return ti ? ti->ti->num_files() : 0; }

const char* tm_tinfo_file_path(tm_tinfo* ti, int index) {
    if (!ti || index < 0 || index >= ti->ti->num_files()) return nullptr;
    std::string p = file_list(*ti->ti).file_path(lt::file_index_t(index));
    std::replace(p.begin(), p.end(), '\\', '/');
    return ret_str(std::move(p));
}

int64_t tm_tinfo_file_size(tm_tinfo* ti, int index) {
    if (!ti || index < 0 || index >= ti->ti->num_files()) return 0;
    return file_list(*ti->ti).file_size(lt::file_index_t(index));
}

int64_t tm_tinfo_total_size(tm_tinfo* ti) { return ti ? ti->ti->total_size() : 0; }

}  // extern "C"
