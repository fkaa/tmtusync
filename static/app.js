function setupHls(video) {
    if (Hls.isSupported()) {
        var hls = new Hls();
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
    this.fold = fold;
    this.playpause = document.getElementById("playpause");
    this.progress = document.getElementById("progress");
    this.progressBar = document.getElementById("progress-bar");

    this.muteButton = document.getElementById("mute");
    this.expandButton = document.getElementById("expand");
    this.volumeContainer = document.getElementById("volume-container");
    this.volumeSlider = document.getElementById("volume-slider");

    this.video.addEventListener("timeupdate", this.OnTimeUpdate.bind(this));
    this.playpause.addEventListener("click", this.OnPlayClick.bind(this));
    this.muteButton.addEventListener("click", this.OnMuteClick.bind(this));
    this.expandButton.addEventListener("click", this.OnExpandClick.bind(this));

    this.progress.addEventListener("click", this.OnTimelineClick.bind(this));
    this.volumeContainer.addEventListener("click", this.OnVolumeClick.bind(this));
}

Controls.prototype.OnTimeUpdate = function(e) {
    if (!this.progress.getAttribute('max')) {
        this.progress.setAttribute('max', this.video.duration);
    }

    this.progress.value = this.video.currentTime;
    this.progressBar.style.width = ((this.video.currentTime / this.video.duration) * 100) + '%';
}

Controls.prototype.OnPlayClick = function(e) {
    if (!this.video.paused) {
        this.room.RequestPause();
        this.playpause.setAttribute("data-state", "pause");
    } else {
        this.room.RequestPlay();
        this.playpause.setAttribute("data-state", "play");
    }
}

Controls.prototype.OnExpandClick = function(e) {
    var state = this.expandButton.getAttribute("data-state");

    if (state == "fullscreen") {
        this.expandButton.setAttribute("data-state", "window");
        this.fold.setAttribute("data-state", "window");
    } else {
        this.expandButton.setAttribute("data-state", "fullscreen");
        this.fold.setAttribute("data-state", "fullscreen");
    }
}

Controls.prototype.OnTimelineClick = function(e) {
    console.log(e);
    console.log("l:"+e.target.offsetLeft+ " w:"+e.target.offsetWidth);
    var pos = (e.pageX - this.progress.offsetLeft) / this.progress.offsetWidth;

    this.room.RequestSeek(pos * this.video.duration);
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

Participant = function(user_id, name, is_me) {
    this.user_id = user_id;
    this.name = name;
}

Room = function(video, title) {
    this.video = video;
    this.title = title;
    this.hls = setupHls(video);
    this.participants = [];
    this.blockEvents = false;
    this.inSeek = false;

    var host = window.location.host;

    this.ws = new WebSocket("ws://" + host + "/room/ws/ElectricBananaBand");
    this.ws.onopen = this.OnWsOpen.bind(this);
    this.ws.onmessage = this.OnWsMessage.bind(this);

    this.video.onplay = this.OnPlay.bind(this);
    this.video.onpause = this.OnPaused.bind(this);
    this.video.onseeked = this.OnSeeked.bind(this);
    this.video.onseeking = this.OnSeeking.bind(this);
}

Room.prototype.RequestPlay = function() {
    console.log("Sending play event");

    this.Send({SetState:{state:"Play"}});
    this.video.play();
}

Room.prototype.RequestPause = function() {
    console.log("Sending pause event");

    this.Send({SetState:{state:"Pause"}});
    this.video.pause();
}

Room.prototype.RequestSeek = function(duration) {
    console.log("Sending seek event: " + duration);

    this.Send({Seek:{duration:duration}});
    this.video.currentTime = duration;
}


Room.prototype.OnWsOpen = function(event) {
    console.log("Connected to room websocket!");

    this.Send({Hello:{name:"bob"}});
}

Room.prototype.OnWsMessage = function(event) {
    var message = JSON.parse(event.data);

    console.log(message);

    if (message.RoomState != null) {
        this.OnRoomState(message.RoomState);
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

        this.title.innerText = stream.name;

        var streamUrl = "/static/data/" + slug + "/" + stream.streams[1].playlist;
        console.log("Loading url: " + streamUrl);

        this.hls.loadSource(streamUrl);
    }

    for (var user in state.participants) {
        this.AddUser(user.user_id, user.name);
    }
}


Room.prototype.OnDoSeek = function(seek) {
    console.log("Received seek message: " + seek.duration);

    this.blockEvents = true;
    this.inSeek = true;
    this.video.currentTime = seek.duration;
}
Room.prototype.OnSetState = function(state) {
    console.log("Received "+state.state+" message");

    switch (state.state) {
        case "Play":
            this.blockEvents = true;
            this.video.play();
            break;
        case "Pause":
            this.blockEvents = true;
            this.video.pause();
            break;
        default:
            console.warn("Unknown video state: " + state.state);
            break;
    }
}


Room.prototype.AddUser = function(user_id, name) {
    this.participants.push(new Participant(user_id, name));

    this.UpdateUserList();
}
Room.prototype.RemoveUser = function(user_id) {
    var idx = this.participants.findIndex((p) => p.user_id == user_id);

    if (idx >= 0) {
        this.participants.splice(idx);

        this.UpdateUserList();
    }
}

Room.prototype.UpdateUserList = function() {

}

Room.prototype.OnNewParticipant = function(participant) {
    this.AddUser(participant.user_id, participant.name);
}

Room.prototype.OnByeParticipant = function(participant) {
    this.RemoveUser(participant.user_id);
}

Room.prototype.OnPlay = function(event) {
    /*if (this.blockEvents || this.inSeek) {
        this.blockEvents = false;
        console.log("Blocked play message");
        return;
    }

    console.log("Sending play message");
    this.Send({SetState:{state:"Play"}});*/
}

Room.prototype.OnPaused = function(event) {
    /*if (this.blockEvents || this.inSeek) {
        this.blockEvents = false;
        console.log("Blocked pause message");
        return;
    }

    console.log("Sending pause message");
    this.Send({SetState:{state:"Pause"}});*/
}

Room.prototype.OnSeeked = function(event) {
    /*if (this.blockEvents || this.inSeek) {
        this.blockEvents = false;
        this.inSeek = false;
        console.log("Blocked seeked message");
        return;
    }
    console.log("Sending seek message: " + video.currentTime);
    this.Send({Seek:{duration:video.currentTime}});*/
}

Room.prototype.OnSeeking = function(event) {
    //this.inSeek = true;
    //console.log("Setting seeking to true");
}

Room.prototype.Send = function(msg) {
    this.ws.send(JSON.stringify(msg));
}

var video = document.getElementById('video-player');
var title = document.getElementById('room-name');
var fold = document.getElementById('video-fold');

var room = new Room(video, title);

var controls = new Controls(video, room, fold);
