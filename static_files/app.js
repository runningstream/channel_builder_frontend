$(document).ready(main);

function main() {
    const SCREEN_AREA = $("#screen_area");

    // See if this is a registration request
    const urlParams = new URLSearchParams(window.location.search);
    const val_code = urlParams.get("val_code");
    if( val_code !== null ) {
        let val_screen = new ValidationScreen();
        val_screen.draw(SCREEN_AREA);
        return;
    }

    // Determine if we already have a valid session id, and if so jump to the main screen
    $.ajax( "/api/v1/validate_session_fe", {
        "method": "GET",
    }).done( function() {
        // Display the main screen
        let main_screen = new MainScreen();
        main_screen.draw(SCREEN_AREA);
    }).fail( function() {
        // Display the login screen
        let login_screen = new LoginScreen();
        login_screen.draw(SCREEN_AREA);
    });
}

function UIScreen() {
    
}

UIScreen.prototype.draw = function (draw_area) {
    let content = $("You drew the parent UIScreen - you should never see this.");
    draw_area.empty();
    draw_area.append(content);
}

function LoginScreen() {
    UIScreen.call(this);
}

LoginScreen.prototype = Object.create(UIScreen.prototype);
Object.defineProperty(LoginScreen.prototype, 'constructor', {
    value: LoginScreen,
    enumerable: false,
    writable: true 
});

LoginScreen.prototype.draw = function (draw_area) {
    let content = $( '<form>' +
        '<label for="login_username">Username:</label>' +
        '<input type="text" id="login_username" autocomplete="username">' +
        '<label for="login_password">Password:</label>' +
        '<input type="password" id="login_password" autocomplete="current-password">' +
        '</form>' );
    let login_button = $( '<input type="button" value="Login">' )
        .click(function() { 
            let login_dat = {
                "username": $("#login_username").val(),
                "password": $("#login_password").val(),
            };
            $.ajax( "/api/v1/authenticate_fe", {
                "method": "POST",
                "data": login_dat,
            }).done(function() {
                // TODO determine if we now have a session id, and if so, jump to the next part, otherwise display failure
                let main_screen = new MainScreen();
                main_screen.draw(draw_area);
            }).fail(function() {
                // TODO determine reason for failure and take action
            });
        });
    let register_button = $( '<input type="button" value="Register">' )
        .click(function() {
            let reg_screen = new RegisterScreen();
            reg_screen.draw(draw_area);
        });

    content.append(login_button);
    content.append(register_button);

    draw_area.empty();
    draw_area.append(content);
}

function RegisterScreen() {
    UIScreen.call(this);
}

RegisterScreen.prototype = Object.create(UIScreen.prototype);
Object.defineProperty(RegisterScreen.prototype, 'constructor', {
    value: RegisterScreen,
    enumerable: false,
    writable: true 
});

RegisterScreen.prototype.draw = function (draw_area) {
    let content = $( '<form>' +
        '<label for="reg_username">Email Address:</label>' +
        '<input type="text" id="reg_username" autocomplete="username">' +
        '<label for="reg_password">Password:</label>' +
        '<input type="password" id="reg_password" autocomplete="current-password">' +
        '<label for="verify_password">Verify Password:</label>' +
        '<input type="password" id="verify_password" autocomplete="current-password">' +
        '</form>'
    );
    let login_button = $( '<input type="button" value="Register">' )
        .click(function() { 
            if( $("#reg_password").val() != $("#verify_password").val() ) {
                // TODO make this nicer
                alert("Passwords do not match!");
                return;
            }
            let reg_dat = {
                "username": $("#reg_username").val(),
                "password": $("#reg_password").val(),
            };
            $.ajax( "/api/v1/create_account", {
                "method": "POST",
                "data": reg_dat,
            }).done(function() {
                // TODO determine whether it was successful and display a message
                alert("User account requested, look for an email...");
                let login_screen = new LoginScreen();
                login_screen.draw(draw_area);
            }).fail(function() {
                // TODO determine reason for failure and take action
            });
        });
    content.append(login_button);

    draw_area.empty();
    draw_area.append(content);
}

function ValidationScreen() {
    UIScreen.call(this);
}

ValidationScreen.prototype = Object.create(UIScreen.prototype);
Object.defineProperty(ValidationScreen.prototype, 'constructor', {
    value: ValidationScreen,
    enumerable: false,
    writable: true 
});

ValidationScreen.prototype.draw = function (draw_area) {
    let content = $( '<form>' +
        '</form>'
    );
    let validate_button = $( '<input type="button" value="Validate Account">' )
        .click(function() {
            const urlParams = new URLSearchParams(window.location.search);
            const val_code = urlParams.get("val_code");
            
            $.ajax( "/api/v1/validate_account?val_code="+val_code, {
                "method": "GET",
            }).done( function() {
                // TODO determine whether it was successful and display a message
                alert("User validation successful!  Now login."); 
                let login_screen = new LoginScreen();
                login_screen.draw(draw_area);
            }).fail( function() {
                // TODO determine reason for failure and take action
            });
        });

    content.append(validate_button);

    draw_area.empty();
    draw_area.append(content);
}

function MainScreen() {
    UIScreen.call(this);
}

MainScreen.prototype = Object.create(UIScreen.prototype);
Object.defineProperty(MainScreen.prototype, 'constructor', {
    value: MainScreen,
    enumerable: false,
    writable: true 
});

MainScreen.prototype.draw = function (draw_area) {
    let content = $( "<div></div>" );
    let mgmt_button_area = $( "<div></div>" );
    let channel_list_area = $( "<div></div>" );
    let channel_edit_area = $( "<div></div>" );

    let channel_list_list = new ChannelListList(channel_list_area, channel_edit_area);
    channel_list_list.draw_channel_list_list();

    let validate_button = $( '<input type="button" value="Validate Session">' )
        .click(function() {
            $.ajax( "/api/v1/validate_session_fe", {
                "method": "GET",
            }).done( function() {
                alert("Session validation successful."); 
            }).fail( function() {
                alert("Fail");
            });
        });

    let logout_button = $( '<input type="button" value="Logout">' )
        .click(function() {
            $.ajax( "/api/v1/logout_session_fe", {
                "method": "GET",
            }).done( function() {
                let login_screen = new LoginScreen();
                login_screen.draw(draw_area);
            }).fail( function() {
                validate_session_or_login_screen(draw_area);
            });
        });

    mgmt_button_area.append(validate_button);
    mgmt_button_area.append(logout_button);

    content.append(mgmt_button_area);
    content.append(channel_list_area);
    content.append(channel_edit_area);

    draw_area.empty();
    draw_area.append(content);
}

function ChannelListList(channel_list_list_area, channel_list_edit_area) {
    this.channel_list_list_area = channel_list_list_area;
    this.channel_list_edit_area = channel_list_edit_area;
    this.channel_list_list = [];
    this.get_channel_lists_from_server();
}

ChannelListList.prototype.get_channel_lists_from_server = function () {
    let channellistlist = this;
    $.ajax( "/api/v1/get_channel_lists", {
        "method": "GET",
    }).done( function(data_str) {
        channellistlist.channel_list_list = JSON.parse(data_str);
        channellistlist.draw_channel_list_list();
    }).fail( function() {
        // TODO improve
        alert("Getting channel lists failed, please refresh");
    });
}

ChannelListList.prototype.draw_channel_list_list = function () {
    let channel_list_list = this;

    let channel_list = $( "<ul></ul>" );

    let new_channel_list_area = $( "<form>" +
        '<label for="new_channel_list_name">New Channel List Name:</label>' +
        '<input type="textarea" id="new_channel_list_name">' +
        "</form>" );

    let create_channel_list_button = 
        $( '<input type="button" value="Create Channel List">' )
        .click(function() {
            let data = {
                "listname": $("#new_channel_list_name").val(),
            };
            channel_list_list.channel_list_list.push(data["listname"]);
            $.ajax( "/api/v1/create_channel_list", {
                "method": "POST",
                "data": data,
            }).done( function() {
                alert("Channel list created."); 
            }).fail( function() {
                alert("Fail");
            });
            channel_list_list.draw_channel_list_list();
        });

    new_channel_list_area.append(create_channel_list_button);

    this.channel_list_list.forEach(function (channel_name) {
        let channel = $( "<li></li>" ).text(channel_name);
        let channellist = new ChannelList(channel_name, channel_list_list.channel_list_edit_area);
        channel.click(function() {
            channellist.draw_channel_list();
        });
        channel_list.append(channel);
    });

    const draw_area = this.channel_list_list_area;
    draw_area.empty();
    draw_area.append(channel_list);
    draw_area.append(new_channel_list_area);
}

function ChannelList(channel_name, channel_list_edit_area) {
    this.channel_name = channel_name;
    this.channel_list_edit_area = channel_list_edit_area;
    this.channel_list = {"entries": []};
    this.get_channel_list_from_server();
}

ChannelList.prototype.get_channel_list_from_server = function () {
    let channellist = this;
    $.ajax( "/api/v1/get_channel_list?list_name="+this.channel_name, {
        "method": "GET",
    }).done( function(data_str) {
        channellist.channel_list = JSON.parse(data_str);
        channellist.draw_channel_list();
    }).fail( function() {
        // TODO improve
        alert("Getting channel list failed, please refresh");
    });
}

ChannelList.prototype.put_channel_list_to_server = function() {
    let channellist = this;
    let list_dat = {
        "listname": this.channel_name,
        "listdata": JSON.stringify(this.channel_list),
    };
    $.ajax( "/api/v1/set_channel_list", {
        "method": "POST",
        "data": list_dat,
    }).done( function(data_str) {
        console.log("Successful update");
    }).fail( function() {
        // TODO improve
        alert("Updating channel list failed, please refresh");
    });
}


ChannelList.prototype.draw_channel_list = function() {
    let channellist = this;
    channellist.channel_list.type="sublist";
    channellist.channel_list.name=channellist.channel_name;
    let channel_edit_list = $( "<div></div>" );

    let recursive_render = function(entry, cur_disp_pos) {
        let ent_disp = $( "<div></div>" );
        ent_disp.append( $("<div></div>").text(entry.name));
        if( entry.type == "sublist" ) {
            let sublist_area = $("<div></div>");
            let ent_button_area = $("<div></div>");
            let new_sublist_name = $('<input type="textarea">');
            let create_sublist_button = $( '<input type="button" value="Create Sublist">' )
                .click(function() {
                    // create the sublist and add it to the common data store of the list
                    channellist.add_sublist(new_sublist_name.val(), entry);
                    // use the api call to store the new version of the list
                    channellist.put_channel_list_to_server();
                    // re-draw the list
                    channellist.draw_channel_list();
                });
            entry.entries.forEach(function (subentry) {
                recursive_render(subentry, sublist_area);
            });
            let new_sublist_label = $('<label>New Sublist Name: </label>');
            new_sublist_label.append(new_sublist_name);

            let new_video_name = $('<input type="textarea">');
            let create_video_button = $( '<input type="button" value="Create Video">' )
                .click(function() {
                    // create the video and add it to the common data store of the list
                    channellist.add_video(new_video_name.val(), entry);
                    // use the api call to store the new version of the list
                    channellist.put_channel_list_to_server();
                    // re-draw the list
                    channellist.draw_channel_list();
                });
            let new_video_label = $('<label>New Video Name: </label>');
            new_video_label.append(new_video_name);

            ent_button_area.append(new_sublist_label);
            ent_button_area.append(create_sublist_button);
            ent_button_area.append(new_video_label);
            ent_button_area.append(create_video_button);

            ent_disp.append(ent_button_area);
            ent_disp.append(sublist_area);
        } else if( entry.type = "video" ) {
            // TODO
        }
        cur_disp_pos.append(ent_disp);
    };

    recursive_render(this.channel_list, channel_edit_list);

    const draw_area = channellist.channel_list_edit_area;
    draw_area.empty();
    draw_area.append(channel_edit_list);
}

ChannelList.prototype.add_sublist = function (sublist_name, entry_position) {
    let entry = {
        "name": sublist_name,
        "image": "",
        "type": "sublist",
        "entries": [],
    };
    entry_position.entries.push(entry);
}

ChannelList.prototype.add_video = function (video_name, entry_position) {
    let entry = {
        "name": video_name,
        "image": "",
        "type": "video",
        "videourl": [],
        "videotype": "mp4",
    };
    entry_position.entries.push(entry);
}

function validate_session_or_login_screen(draw_area) {
    $.ajax( "/api/v1/validate_session_fe", {
        "method": "GET",
    }).done( function() {
        // Do nothing - we're still valid
    }).fail( function() {
        let login_screen = new LoginScreen();
        login_screen.draw(draw_area);
    });
}
