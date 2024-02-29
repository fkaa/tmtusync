function createBadge(id) {
    var i = document.createElement('i');
    i.classList.add("sprite");
    i.classList.add("sprite-"+BADGE_DATA[id].name);
    i.setAttribute("data-toggle", "tooltip");
    i.setAttribute("title", BADGE_DATA[id].tooltip);
    return i;
}

function time() {
    return Date.now();
}

function secondsToTime(seconds) {
    seconds = Number(seconds);
    var h = Math.floor(seconds / 3600);
    var m = Math.floor(seconds % 3600 / 60);
    var s = Math.floor(seconds % 60);

    var text = "";
    if (h > 0) {
        text += h + "h ";
    }

    if (h > 0 || m > 0) {
        text += m + "m ";
    }

    return text + s + "s";
    // return h.toString().padStart(2, "0") + ":" + m.toString().padStart(2, "0") + ":" + s.toString().padStart(2, "0");
}

function bufferedFromPosition(video, pos) {
    var bufferedRanges = video.buffered;

    for (var i = 0; i < bufferedRanges.length; i++) {
        var start = video.buffered.start(i);
        var end = video.buffered.end(i);

        if (start <= pos && end >= pos) {
            return end - pos;
        }
    }

    return 0;
}

function setupHls(video) {
    if (Hls.isSupported()) {
        var hls = new Hls({
            debug: false,
            backBufferLength: 0,
            appendErrorMaxRetry: 30,
        });
        hls.on(Hls.Events.ERROR, function (event, data) {
            var errorType = data.type;
            var errorDetails = data.details;
            var errorFatal = data.fatal;

            console.error(errorType + ": " + errorDetails);
        });

        hls.attachMedia(video);

        return hls;
    } else if (video.canPlayType('application/vnd.apple.mpegurl')) {
        video.src = videoSrc;
    }
}

Controls = function(video, room, fold) {
    this.video = video;
    this.room = room;
    this.playpause = document.getElementById("playpause");
    this.progress = document.getElementById("progress");
    this.progressBar = document.getElementById("progress-bar");

    this.muteButton = document.getElementById("mute");
    this.volumeContainer = document.getElementById("volume-container");
    this.volumeSlider = document.getElementById("volume-slider");



    this.volumeContainer.addEventListener("click", this.OnVolumeClick.bind(this));
}

Controls.prototype.OnTimeUpdate = function(e) {
    if (!this.progress.getAttribute('max')) {
        this.progress.setAttribute('max', this.video.duration);
    }

    this.progress.value = this.video.currentTime;
    this.progressBar.style.width = ((this.video.currentTime / this.video.duration) * 100) + '%';
}

Controls.prototype.OnVideoPause = function(e) {
    this.playpause.setAttribute("data-state", "pause");
}

Controls.prototype.OnVideoPlay = function(e) {
    this.playpause.setAttribute("data-state", "play");
}




Controls.prototype.OnMuteClick = function(e) {
    this.video.muted = !this.video.muted;

    this.UpdateVolumeControl();
}

Controls.prototype.OnVolumeClick = function(e) {
    console.log(e);
    var pos = (e.pageX - this.volumeContainer.offsetLeft) / this.volumeContainer.offsetWidth;

    if (pos <= 0.02) {
        this.video.volume = 0;
    } else {
        this.video.volume = pos;
        this.video.muted = false;
    }

    this.UpdateVolumeControl();
}

Controls.prototype.UpdateVolumeControl = function() {
    var vol = this.video.muted ? 0 : this.video.volume;

    if (vol == 0) {
        this.muteButton.setAttribute("data-state", "mute");
    } else {
        this.muteButton.setAttribute("data-state", "volume");
    }

    this.volumeSlider.style.width = Math.floor(vol * 100) + '%';
}

Participant = function(userlist_table, user_id, name, is_me, avatar, badges) {
    this.userlist_table = userlist_table.getElementsByTagName('tbody')[0];
    this.user_id = user_id;
    this.name = name;
    this.avatar = avatar;
    this.badges = badges;

    this.user_row = document.createElement('tr');
    this.avatar_col = document.createElement('td');
    this.name_col = document.createElement('td');
    this.time_col = document.createElement('td');
    this.buffered_col = document.createElement('td');
    this.state_col = document.createElement('td');
    this.badge_col = document.createElement('td');

    this.name_col.innerText = name;
    this.avatar_col.setAttribute("valign", "center");
    this.avatar_col.classList.add("avatar-col");
    this.state_col.setAttribute("valign", "center");
    this.state_col.classList.add("badge-col");
    this.badge_col.setAttribute("valign", "center");
    this.badge_col.classList.add("badge-col");

    this.avatar_col.appendChild(createBadge(avatar));
    this.state_col.appendChild(createBadge(6));

    for (var i = 0; i < badges.length; i++) {
        var badge = badges[i];
        this.badge_col.appendChild(createBadge(badge));
    }

    this.user_row.appendChild(this.avatar_col);
    this.user_row.appendChild(this.name_col);
    this.user_row.appendChild(this.time_col);
    this.user_row.appendChild(this.buffered_col);
    this.user_row.appendChild(this.state_col);
    this.user_row.appendChild(this.badge_col);

    this.userlist_table.appendChild(this.user_row);

    if (!is_me) {
        this.interval = setInterval(this.OnUpdateInterval.bind(this), 1000);
    }

    console.log(this);
}

Participant.prototype.OnUpdateInterval = function() {
    if (this.state == "Play") {
        this.duration += 1.0;

        this.UpdateColumn();
    }
}

Participant.prototype.Update = function(update) {
    // console.log("Setting duration ("+this.duration +") of " + this.user_id + " to " + update.duration);

    this.oldbadges = this.badges;
    this.badges = update.badges;
    this.duration = update.duration;
    this.buffered = update.buffered;
    this.state = update.state;

    this.UpdateColumn();
}

Participant.prototype.UpdateSelf = function(update) {
    // console.log("Setting duration ("+this.duration +") of " + this.user_id + " to " + update.duration);

    this.oldbadges = this.badges;
    this.duration = update.duration;
    this.buffered = update.buffered;
    this.state = update.state;

    this.UpdateColumn();
}

Participant.prototype.UpdateColumn = function() {
    this.time_col.innerText = secondsToTime(this.duration);
    this.buffered_col.innerText = secondsToTime(this.buffered);

    this.state_col.innerHTML = '';
    if (this.state == "Play") {
        this.state_col.appendChild(createBadge(14));
    } else if (this.state == "Pause") {
        this.state_col.appendChild(createBadge(16));
    } else {
        this.state_col.appendChild(createBadge(6));
    }

    if (JSON.stringify(this.oldbadges) !== JSON.stringify(this.badges)) {
        console.log("updating badges for " + this.name);
        this.badge_col.innerHTML = '';
        for (var i = 0; i < this.badges.length; i++) {
            var badge = this.badges[i];
            this.badge_col.appendChild(createBadge(badge));
        }
    }
}

Participant.prototype.Remove = function() {
    this.user_row.remove();
    clearInterval(this.interval);
}

Room = function(video, title, fold, userlist, logcontainer, loglist) {
    this.video = video;
    this.title = title;
    this.userlist = userlist;
    this.logbody = loglist.getElementsByTagName('tbody')[0];
    this.logcontainer = logcontainer;
    this.hls = setupHls(video);
    this.participants = [];
    this.blockEvents = false;
    this.inSeek = false;
    this.self_state = {
        duration: 0,
        buffered: 0,
        state: "Pause",
    };
    this.expandButton = document.getElementById("expand");
    this.expandButton.addEventListener("click", this.OnExpandClick.bind(this));
    this.fold = fold;

    var listeners = {
        play: this.OnPlayClick.bind(this),
        pause: this.OnPauseClick.bind(this),
        seek: this.OnTimelineClick.bind(this),
    };

    this.player = new Plyr(this.video, {debug: false, listeners: listeners});

    this.player.on('play', this.OnPlay.bind(this));
    this.player.on('pause', this.OnPaused.bind(this));
    this.player.on('seeked', this.OnSeeked.bind(this));
    this.player.on('seeking', this.OnSeeking.bind(this));
    this.player.on('timeupdate', this.OnTimeUpdate.bind(this));
    this.player.on('loadeddata ', this.OnVideoLoaded.bind(this));

    /*this.player.addEventListener("timeupdate", this.OnTimeUpdate.bind(this));
    this.player.addEventListener("pause", this.OnVideoPause.bind(this));
    this.player.addEventListener("play", this.OnVideoPlay.bind(this));*/

    //this.playpause.addEventListener("click", this.OnPlayClick.bind(this));
    //this.muteButton.addEventListener("click", this.OnMuteClick.bind(this));
    //this.progress.addEventListener("click", this.OnTimelineClick.bind(this));

    /*this.video.onplay = this.OnPlay.bind(this);
    this.video.onpause = this.OnPaused.bind(this);
    this.video.onseeked = this.OnSeeked.bind(this);
    this.video.onseeking = this.OnSeeking.bind(this);
    this.video.ontimeupdate = this.OnTimeUpdate.bind(this);
    this.video.onloadeddata = this.OnVideoLoaded.bind(this);*/
}

Room.prototype.OnExpandClick = function(e) {
    var state = this.expandButton.getAttribute("data-state");

    if (state == "fullscreen") {
        this.expandButton.setAttribute("data-state", "window");
        this.fold.setAttribute("data-state", "window");
    } else {
        this.expandButton.setAttribute("data-state", "fullscreen");
        this.fold.setAttribute("data-state", "fullscreen");
    }
}

Room.prototype.OnTimelineClick = function(e) {
    console.log(e);
    var pos = parseFloat(e.target.attributes["seek-value"].nodeValue) / 100.0;
    var duration = this.video.duration;

    if (!isNaN(duration) && typeof duration == 'number') {
        this.RequestSeek(pos * duration);
    }
}

Room.prototype.OnPlayClick = function(e) {
    if (this.player.paused) {
        this.RequestPlay();
    } else {
        this.RequestPause();
    }
    // this.video.play();
}

Room.prototype.OnPauseClick = function(e) {
    this.RequestPause();
    this.video.pause();
}

Room.prototype.Log = function(src, msg) {
    var tr = document.createElement('tr');

    var tdSrc = document.createElement('td');
    var tdMsg = document.createElement('td');

    if (src != null) {
        tdSrc.appendChild(createBadge(src.avatar));
    } else {
        tdSrc.appendChild(createBadge(9));
    }

    msg = msg.replaceAll('{}', '<span class="user-name">' + src.name + '</span>');

    tdMsg.innerHTML = msg;

    tr.appendChild(tdSrc);
    tr.appendChild(tdMsg);

    this.logbody.appendChild(tr);
    this.logcontainer.scrollTo(0, this.logcontainer.scrollHeight);
}

Room.prototype.Connect = function() {
    this.self_user = new Participant(this.userlist, null, USERNAME, true, AVATAR, BADGES)
    this.participants = [this.self_user];
    this.username = USERNAME;

    var host = window.location.host;

    this.ws = new WebSocket("ws://" + host + "/websocket/" + ROOM_CODE);
    this.ws.onopen = this.OnWsOpen.bind(this);
    this.ws.onmessage = this.OnWsMessage.bind(this);
}

Room.prototype.RequestPlay = function() {
    this.Send({SetState:{
        state:"Play",
        time: time(),
    }});

    this.participants.forEach(p => p.state = "Play");

    console.log("Sent play event");

    // this.video.play();
}

Room.prototype.RequestPause = function() {
    this.Send({SetState:{
        state:"Pause",
        time: time(),
    }});

    this.participants.forEach(p => p.state = "Pause");

    console.log("Sent pause event");

    // TODO: maybe we want to only let server pause clients videos? scary
    // this.video.pause();
}

Room.prototype.RequestSeek = function(duration) {
    this.Send({Seek:{
        duration:duration,
        time: time(),
    }});

    console.log("Sent seek event: " + duration);

    this.video.currentTime = duration;
}

Room.prototype.SetTime = function(duration) {
    this.current_time_set = time();
    this.current_time = duration;
}

Room.prototype.SetState = function(playing) {
    this.current_state_set = time();
    this.current_state = playing;
}

Room.prototype.OnWsOpen = function(event) {
    console.log("Connected to room websocket!");

    this.Send({
        Hello:{
            name: this.username,
            avatar: AVATAR,
            time: time(),
        }
    });
}

Room.prototype.OnWsMessage = function(event) {
    var message = JSON.parse(event.data);

    // console.log(message);

    if (message.RoomState != null) {
        this.OnRoomState(message.RoomState);
    } else if (message.RoomUpdate != null) { // room has updated
        this.OnRoomUpdate(message.RoomUpdate);
    } else if (message == "Ping") { // ping!
        this.OnPing();
    } else if (message.NewParticipant != null) { // new participant
        this.OnNewParticipant(message.NewParticipant);
    } else if (message.ByeParticipant != null) { // participant left
        this.OnByeParticipant(message.ByeParticipant);
    } else if (message.DoSeek != null) { // someone seeked
        this.OnDoSeek(message.DoSeek);
    } else if (message.SetState != null) { // someone changed state
        this.OnSetState(message.SetState);
    }
}

Room.prototype.OnRoomState = function(state) {
    var stream = state.current_stream;
    if (stream != null) {
        var slug = stream.slug;

        var streamUrl = "/static/data/" + slug + "/" + stream.streams[0].playlist;
        console.log("Loading url: " + streamUrl);
        this.startingDuration = stream.duration;
        this.startingState = stream.state;

        this.hls.loadSource(streamUrl);
    }

    this.SetTime(0);
    this.SetState(false);


    var p = this.participants.find((p) => p.user_id == null);
    p.user_id = state.user_id;

    state.participants.forEach(p => {
        this.AddUser(p.user_id, p.name, p.avatar, p.badges);
    });
}

Room.prototype.OnRoomUpdate = function(update) {
    update.participants.forEach(update => {
        var participant = this.participants.find((p) => p.user_id == update.user_id);

        if (participant != null) {
            participant.Update(update);
        }
    });
}

Room.prototype.UpdateSelf = function() {
    var duration = this.video.currentTime;
    var buffered = bufferedFromPosition(this.video, duration);

    var state = this.current_state ? "Play" : "Pause";

    this.self_state = {
        duration: duration,
        buffered: buffered,
        state: state
    };

    this.self_user.UpdateSelf(this.self_state);
}

Room.prototype.OnPing = function(seek) {
    // console.log("Received ping message");

    this.UpdateSelf();

    var duration = this.video.currentTime;
    var buffered = bufferedFromPosition(this.video, duration);
    var state = this.current_state ? "Play" : "Pause";

    this.Send({State:{
        duration: this.current_time,
        duration_time: this.current_time_set,
        state: state,
        state_time: this.current_state_set,
        buffered: buffered,
        time: time(),
    }});
}

Room.prototype.OnDoSeek = function(seek) {
    console.log("Received seek message: " + seek.duration);

    var src = this.participants.find((p) => p.user_id == seek.user);
    this.Log(src, "{} requested to seek to " + secondsToTime(seek.duration) + ".");

    this.blockEvents = true;
    this.inSeek = true;
    this.video.currentTime = seek.duration;
}

Room.prototype.OnSetState = function(state) {
    console.log("Received "+state.state+" message");
    var src = this.participants.find((p) => p.user_id == state.user);

    switch (state.state) {
        case "Play":
            this.Log(src, "{} requested to play.");
            this.blockEvents = true;
            this.video.play();
            break;
        case "Pause":
            this.Log(src, "{} requested to pause.");
            this.blockEvents = true;
            this.video.pause();
            break;
        default:
            console.warn("Unknown video state: " + state.state);
            break;
    }
}


Room.prototype.AddUser = function(user_id, name, avatar, badges) {
    console.log("Adding user "+name+"#"+user_id);

    p = this.participants.find((p) => p.user_id == user_id);

    if (p == null) {
        console.log("Adding new");
        var participant = new Participant(this.userlist, user_id, name, false, avatar, badges);
        this.participants.push(participant);

        this.UpdateUserList();
        return participant;
    }
}

Room.prototype.RemoveUser = function(user_id) {
    var idx = this.participants.findIndex((p) => p.user_id == user_id);

    if (idx >= 0) {
        var participant = this.participants.splice(idx)[0];
        console.log("Removing user "+participant.name+"#"+user_id);
        participant.Remove();

        this.UpdateUserList();
    } else {
        console.warn("Failed to find existing user for leaving user id #"+user_id);
    }
}

Room.prototype.UpdateUserList = function() {

}

Room.prototype.OnNewParticipant = function(p) {
    var p = this.AddUser(p.user_id, p.name, p.avatar, p.badges);

    if (p != null) {
        this.Log(p, "{} has joined the room!");
    }
}

Room.prototype.OnByeParticipant = function(p) {
    this.RemoveUser(p.user_id);
}

Room.prototype.OnVideoLoaded = function(event) {
    console.log("Video loaded. Starting time: " + this.startingDuration);

    if (this.startingDuration != null) {
        this.video.currentTime = this.startingDuration;
    }

    if (this.startingState == "Play") {
        this.video.play();
    }
}

Room.prototype.OnTimeUpdate = function(event) {
    this.SetTime(video.currentTime);

    this.self_state.duration = video.currentTime;
    this.UpdateSelf();
}

Room.prototype.OnPlay = function(event) {
    this.SetState(true);

    this.self_state.state = "Play";
    this.UpdateSelf();
}

Room.prototype.OnPaused = function(event) {
    this.SetState(false);

    this.self_state.state = "Pause";
    this.UpdateSelf();
}

Room.prototype.OnSeeked = function(event) {
}

Room.prototype.OnSeeking = function(event) {
}

Room.prototype.Send = function(msg) {
    this.ws.send(JSON.stringify(msg));
}

var video = document.getElementById('video-player');
var title = document.getElementById('room-name');
var fold = document.getElementById('video-fold');
var userlist = document.getElementById('userlist');
var loglist = document.getElementById('logcontainer');
var loglist = document.getElementById('loglist');

var room = new Room(video, title, fold, userlist, logcontainer, loglist);
//var controls = new Controls(video, room, fold);

room.Connect();

