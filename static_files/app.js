$(document).ready(main);

function main() {

    let screen_props = {
        head_area: $("#header_area"),
        draw_area: $("#screen_area"),
        foot_area: $("#footer_area"),
        pops_back: $("#popup_back"),
        pops_area: $("#popup_area"),
    };

    const urlParams = new URLSearchParams(window.location.search);

    // See if this is a registration request
    const val_code = urlParams.get("val_code");
    if( val_code !== null ) {
        let val_screen = new ValidationScreen();
        val_screen.draw(screen_props);
        return;
    }

    // See if this is the signup page
    if( urlParams.get("signup") != null ) {
        let reg_screen = new RegisterScreen();
        reg_screen.draw(screen_props);
        return;
    }

    // Determine if we already have a valid session id, and if so jump to the main screen
    $.ajax( get_api_url("validate_session_fe"),
        get_api_properties({"method": "GET"})
    ).done( function() {
        // Display the main screen
        let main_screen = new MainScreen();
        main_screen.draw(screen_props);
    }).fail( function() {
        // Display the login screen
        let login_screen = new LoginScreen();
        login_screen.draw(screen_props);
    });
}

function UIScreen() {
    
}

UIScreen.prototype.clear = function (screen_props) {
    $("body")[0].className = "";
    screen_props.head_area.empty();
    screen_props.foot_area.empty();
    screen_props.draw_area.empty();
    screen_props.pops_area.empty();
}

UIScreen.prototype.draw = function (screen_props) {
    this.clear(screen_props);
    draw_area.append($("You drew the UIScreen.  That should never happen."));
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

LoginScreen.prototype.draw = function (screen_props) {
    this.clear(screen_props);
    $("body").addClass("login_page");

    let intro_section = $( 
        '<section id="login_intro">' +
        '</section>'
    );
    let intro_text = $( 
        '<div id="login_intro_text">' +
        '<h1>Running Stream</h1>' +
        '<h2>Your Personal Roku Channel</h2>' +
        '<br/>' +
        '<a href="/?signup=" class="input_button">Sign Up</a>' +
        '</div>'
    );
    let usecases_section = $(
        '<section id="login_usecases">' +
        '<h1>Why?</h1>' +
        '<div class="usecase">' +
        '<img src="img/videotape.svg" />' +
        '<p>Home videos on demand</p>' +
        '</div>' +
        '<div class="usecase">' +
        '<img src="img/musicparty.svg" />' +
        '<p>Stream your party mix!</p>' +
        '</div>' +
        '<div class="usecase">' +
        '<img src="img/baby.svg" />' +
        '<p>Build a channel just for your child</p>' +
        '</div>' +
        '<div class="usecase">' +
        '<img src="img/mortarboard.svg" />' +
        '<p>Put a graduate compilation on loop</p>' +
        '</div>' +
        '<div class="usecase">' +
        '<img src="img/shop.svg" />' +
        "<p>Loop your shop's promo video</p>" +
        '</div>' +
        '</section>'
    );
    let trailer_section = $(
        '<section id="login_trailer">' +
        '<h2>Free.  No catch.</h2>' +
        '<h3><a href="https://github.com/runningstream/channel_builder">Source Code Here</a></h3>' +
        "<p>Why?  We're nerds and we use this ourselves.  It doesn't cost us much to let others in too.</p>" +
        '</section>'
    );
    let docs_section = $(
        '<section id="login_docs">' +
        '<h2>Documentation</h2>' +
        '<h3><a href="https://docs.runningstream.cc/">Getting Started</a></h3>' +
        '</section>'
    );
    let login_section = $(
        '<section id="login_login">' +
        '</section>'
    );
    let login_form = $(
        '<form>' +
        '<div>' +
        '<label for="login_username">Username:</label>' +
        '<input type="text" id="login_username" autocomplete="username">' +
        '</div>' +
        '<div>' +
        '<label for="login_password">Password:</label>' +
        '<input type="password" id="login_password" autocomplete="current-password">' +
        '</div>' +
        '</form>' )
        .submit(function() {
            let login_dat = {
                "username": $("#login_username").val(),
                "password": $("#login_password").val(),
            };
            $.ajax( get_api_url("authenticate_fe"),
                get_api_properties({"method": "POST", "data": login_dat})
            ).done(function() {
                // TODO determine if we now have a session id, and if so, jump to the next part, otherwise display failure
                let main_screen = new MainScreen();
                main_screen.draw(screen_props);
            }).fail(function(jqXHR, text_status, asdf) {
                display_popup(screen_props, "Login Failed");
            });

            // Return false to cancel the submit action
            return false;
        });
    let login_button = $( '<input type="submit" value="Login">' );
    let register_button = $(
        '<a href="/?signup=" class="input_button">Register</a>'
        );

    let button_div = $('<div></div>');
    button_div.append(login_button);
    button_div.append(register_button);
    login_form.append(button_div);
    intro_section.append(intro_text);
    login_section.append(login_form);

    screen_props.draw_area.append(intro_section);
    screen_props.draw_area.append(login_section);
    console.log(usecases_section);
    screen_props.draw_area.append(usecases_section);
    screen_props.draw_area.append(trailer_section);
    screen_props.draw_area.append(docs_section);
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

RegisterScreen.prototype.draw = function (screen_props) {
    this.clear(screen_props);
    $("body").addClass("register_page");

    let intro_content = $( 
        '<section>' +
        '<h1>Sign up now!</h1>' +
        '</section>'
    );
    let trailer_section = $(
        '<section>' +
        '<p>Privacy policy: we will never give anyone else your information.  You will only ever receive a signup confirmation email from us.</p>' +
        '</section>'
    );
    let register_form = $( 
        '<form>' +
        '<div>' +
        '<label for="reg_username">Email Address:</label>' +
        '<input type="text" id="reg_username" autocomplete="username">' +
        '</div>' +
        '<div>' +
        '<label for="reg_password">Password:</label>' +
        '<input type="password" id="reg_password" autocomplete="current-password">' +
        '</div>' +
        '<div>' +
        '<label for="verify_password">Verify Password:</label>' +
        '<input type="password" id="verify_password" autocomplete="current-password">' +
        '</div>' +
        '</form>'
    )
        .submit(function() { 
            if( $("#reg_password").val() != $("#verify_password").val() ) {
                display_popup(screen_props, "Passwords do not match!");
                return false;
            }
            let reg_dat = {
                "username": $("#reg_username").val(),
                "password": $("#reg_password").val(),
            };
            $.ajax( get_api_url("create_account"),
                get_api_properties({"method": "POST", "data": reg_dat})
            ).done(function() {
                // TODO determine whether it was successful and display a message
                display_popup(screen_props,
                    "User account requested, look for an email...",
                    function() {
                        let login_screen = new LoginScreen();
                        login_screen.draw(screen_props);
                    }
                );
            }).fail(function() {
                // TODO determine reason for failure and take action
                display_popup(screen_props,
                    "User account creation failed!");
            });
            return false;
        });
    let register_section = $(
        '<section>' +
        '</section>'
    );
    let register_button = $( '<input type="submit" value="Register">' );
    register_form.append(register_button);
    register_section.append(register_form)

    screen_props.draw_area.append(intro_content);
    screen_props.draw_area.append(register_section);
    screen_props.draw_area.append(trailer_section);
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

ValidationScreen.prototype.draw = function (screen_props) {
    this.clear(screen_props);
    $("body").addClass("validation_page");

    let val_welcome = $( '<h2>Welcome to Running Stream!</h2>' +
        '<p>Register here!  If you didn\'t request to register, or you no longer wish to receve registration emails, just delete the one you received.  You will not receive any further communications from this service</p>'
    );
    let val_form = $( '<form>' +
            '</form>')
        .submit(function() {
            const urlParams = new URLSearchParams(window.location.search);
            const val_code = urlParams.get("val_code");

            let goto_login = function () {
                window.history.pushState("", "", "/");
                let login_screen = new LoginScreen();
                login_screen.draw(screen_props);
            };
            
            $.ajax( get_api_url("validate_account?val_code="+val_code),
                get_api_properties({"method": "GET"})
            ).done( function() {
                // TODO determine whether it was successful and display a message
                display_popup(screen_props, "User validation successful!  Now login.",
                    goto_login);
            }).fail( function(jqXHR, textStatus, errorThrown) {
                // TODO test the part below, perhaps display in a nicer way
                if( jqXHR.status == 403 ) {
                    display_popup(screen_props, "Invalid validation code!", goto_login); 
                } else {
                    display_popup(screen_props, "Validation failed.", goto_login); 
                }
            });
            return false;
        });
    let validate_button = $( '<input type="submit" value="Validate Account">' );

    val_form.append(validate_button);

    screen_props.draw_area.append(val_welcome);
    screen_props.draw_area.append(val_form);
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

MainScreen.prototype.draw = function (screen_props) {
    this.clear(screen_props);
    $("body").addClass("mainscreen_page");

    let content = $( '<div id="content"></div>' );
    let mgmt_button_area = $( "<div></div>" );
    let channel_list_area = $( '<div id="chan_list"></div>' );
    let channel_edit_area = $( '<div id="chan_edit"></div>' );

    let channel_list_list = new ChannelListList(screen_props, channel_edit_area);
    channel_list_list.draw(channel_list_area);

    let validate_button = $( '<input type="button" value="Validate Session">' )
        .click(function() {
            $.ajax( get_api_url("validate_session_fe"),
                get_api_properties({"method": "GET"})
            ).done( function() {
                display_popup(screen_props, "Session validation successful."); 
            }).fail( function() {
                display_popup(screen_props, "Session validation failed!"); 
            });
        });

    let logout_button = $( '<div id="logout_button"></div>' )
        .click(function() {
            $.ajax( get_api_url("logout_session_fe"),
                get_api_properties({"method": "GET"})
            ).done( function() {
                let login_screen = new LoginScreen();
                login_screen.draw(screen_props);
            }).fail( function() {
                validate_session_or_login_screen(screen_props);
            });
        });

    let profile_area = $('<div id="profile_button"></div>');

    mgmt_button_area.append(validate_button);
    mgmt_button_area.append(logout_button);

    mgmt_button_area.append(profile_area);

    content.append(channel_list_area);
    content.append(channel_edit_area);

    screen_props.head_area.append(mgmt_button_area);
    screen_props.draw_area.append(content);
}

function ChannelListList(screen_props, channel_list_edit_area) {
    this.channel_list_list_area = $( "<div></div>" );
    this.channel_list_edit_area = channel_list_edit_area;
    this.channel_list_edit_button_dest = screen_props.foot_area;
    this.channel_list_list = [];
    this.screen_props = screen_props;
    this.selected_list = null;

    this.get_channel_lists_from_server(screen_props);
}

ChannelListList.prototype.get_channel_lists_from_server = function (screen_props) {
    let channellistlist = this;
    $.ajax( get_api_url("get_channel_lists"),
        get_api_properties({"method": "GET"})
    ).done( function(data_str) {
        channellistlist.channel_list_list = JSON.parse(data_str);
        channellistlist.draw_channel_list_list();
    }).fail( function() {
        // TODO improve
        display_popup(screen_props, "Getting channel lists failed, please refresh");
    });
}

ChannelListList.prototype.draw = function (draw_area) {
    draw_area.children().detach();
    draw_area.append(this.channel_list_list_area);
}

ChannelListList.prototype.draw_channel_list_list = function () {
    let channel_list_list = this;

    let channel_list = $( '<div id="chan_list_list"></div>' );

    let new_channel_list_area = $( "<form>" +
        '<label for="new_channel_list_name">New Channel List Name:</label>' +
        '<input type="text" id="new_channel_list_name">' +
        "</form>" );


    let create_channel_list_button = 
        $( '<input type="button" value="Create Channel List">' )
        .click(function() {
            let data = {
                "listname": $("#new_channel_list_name").val(),
            };
            channel_list_list.channel_list_list.push(data["listname"]);
            $.ajax( get_api_url("create_channel_list"),
                get_api_properties({"method": "POST", "data": data})
            ).done( function() {
                alert("Channel list created."); 
            }).fail( function() {
                alert("Fail");
            });
            channel_list_list.draw_channel_list_list();
        });

    new_channel_list_area.append(create_channel_list_button);

    let set_active_channel_list_button = 
        $( '<input type="button" value="Set Active List">' )
        .click(function() {
            let curr_sel = channel_list_list.currently_selected;
            if( curr_sel == null ) {
                return;
            }
            let data = {
                "listname": channel_list_list.currently_selected.channel_name,
            };
            console.log(data);
            $.ajax( get_api_url("set_active_channel"),
                get_api_properties({"method": "POST", "data": data})
            ).done( function() {
                alert("Active channel set."); 
            }).fail( function() {
                alert("Fail");
            });
        });
    new_channel_list_area.append(set_active_channel_list_button);

    let chan_edit_butt_dest = this.channel_list_edit_button_dest;

    this.channel_list_list.forEach(function (channel_name) {
        let channel = $( '<div class="sel_list_ent"></div>' ).text(channel_name);
        let channellist = new ChannelList(channel_list_list.screen_props,
            channel_name, chan_edit_butt_dest);
        channel.click(function() {
            channellist.draw(channel_list_list.channel_list_edit_area);
            channel_list_list.set_selection(channellist, channel.get()[0]);
        });
        channel_list.append(channel);
    });

    this.channel_list_list_area.children().detach();
    this.channel_list_list_area.append(channel_list);
    this.channel_list_list_area.append(new_channel_list_area);
}

ChannelListList.prototype.set_selection = function(channel_list, domelem) {
    this.currently_selected = channel_list;
    $(".selected").removeClass("selected");
    domelem.classList.add("selected");
}

function ChannelList(screen_props, channel_name, chan_edit_butt_dest) {
    this.channel_name = channel_name;
    this.channel_list_button_dest = chan_edit_butt_dest;
    this.channel_list_edit_area = $("<div></div>");
    this.channel_list_button_area = $("<div></div>");
    this.channel_list = {"entries": []};
    this.ui_only_entry_props = ["expanded"];
    this.screen_props = screen_props;
    this.currently_selected = null;
    this.add_entry_button = null;
    this.change_entry_button = null;

    this.get_channel_list_from_server();
}

ChannelList.prototype.draw = function (draw_area) {
    draw_area.children().detach();
    draw_area.append(this.channel_list_edit_area);
    this.channel_list_button_dest.children().detach();
    this.channel_list_button_dest.append(this.channel_list_button_area);
}

ChannelList.prototype.get_channel_list_from_server = function () {
    let channellist = this;
    $.ajax( get_api_url("get_channel_list?list_name="+this.channel_name),
        get_api_properties({"method": "GET"})
    ).done( function(data_str) {
        channellist.channel_list = JSON.parse(data_str);
        channellist.draw_channel_list();
    }).fail( function() {
        // TODO improve
        alert("Getting channel list failed, please refresh");
    });
}

ChannelList.prototype.put_channel_list_to_server = function() {
    const to_strip = this.ui_only_entry_props;

    let strip_ui_specific = function(entry) {
        let new_ent = {};
        for( key in entry ) {
            if( key == "entries" ) {
                new_ent[key] = entry.entries.map( strip_ui_specific );
            } else if( ! to_strip.includes(key) ) {
                new_ent[key] = entry[key];
            }
        }
        return new_ent;
    };

    let channellist = this;
    let list_dat = {
        "listname": this.channel_name,
        "listdata": JSON.stringify(strip_ui_specific(this.channel_list)),
    };
    $.ajax( get_api_url("set_channel_list"),
        get_api_properties({"method": "POST", "data": list_dat})
    ).done( function(data_str) {
        console.log("Successful update");
    }).fail( function() {
        // TODO improve
        alert("Updating channel list failed, please refresh");
    });
}

let sublist_dialog_content = function(cur_entry=undefined) {
    if(cur_entry === undefined) {
        cur_entry = {"name": "", "image": ""};
    }
    return $( '<div class="block_labels">' +
        '<label>Name: <input type="text" id="pop_addname" value="' + cur_entry.name + '"></label>' +
        '<label>Image URL: <input type="text" id="pop_imageurl" value="' + cur_entry.image + '"></label>' +
        '</div>' );
}

let media_dialog_content = function(cur_entry=undefined) {
    if(cur_entry === undefined) {
        cur_entry = {"name": "", "image": "", "videourl": "",
            "videotype": "mp4", "loop": false};
    }
    let loop_checked = cur_entry.loop ? " checked" : "";
    let content = $('<div class="block_labels">' +
        '<label>Name: <input type="text" id="pop_addname" value="' + cur_entry.name + '"></label>' +
        '<label>Image URL: <input type="text" id="pop_imageurl" value="' + cur_entry.image + '"></label>' +
        '<label>Media URL: <input type="text" id="pop_videourl" value="' + cur_entry.videourl + '"></label>' +
        '<label>Loop Media: <input type="checkbox" id="pop_loopmedia"' + loop_checked + '></label>' +
        '<label>MP4 <input type="radio" value="mp4" name="pop_videnctype"></label>' +
        '<label>Audio <input type="radio" value="audio" name="pop_videnctype"></label>' +
        '</div>' );
    content.find('input[name="pop_videnctype"][value="' + cur_entry.videotype + '"]').prop('checked', true);
    return content;
}

ChannelList.prototype.draw_channel_list = function() {
    let channellist = this;
    channellist.channel_list.type="sublist";
    channellist.channel_list.name=channellist.channel_name;
    let channel_edit_buttons = $( "<div></div>" );
    let channel_edit_list = $( '<div></div>' );

    let new_name = $('<input type="text" id="medianame">');
    let new_name_label = $('<label>Name: </label>');
    new_name_label.append(new_name);

    let new_imgurl = $('<input type="text" id="imageurl">');
    let new_imgurl_label = $('<label>Image URL: </label>');
    new_imgurl_label.append(new_imgurl);

    let new_vidurl = $('<input type="text" id="mediaurl">');
    let new_vidurl_label = $('<label>Media URL: </label>');
    new_vidurl_label.append(new_vidurl);

    let new_typesub = $('<input type="radio" value="sublist" name="thingtype">');
    let new_typesub_label = $('<label>Sublist</label>');
    new_typesub_label.append(new_typesub);
    let new_typevid = $('<input type="radio" value="video" name="thingtype" checked="true">');
    let new_typevid_label = $('<label>Media</label>');
    new_typevid_label.append(new_typevid);

    let new_loop = $('<input type="checkbox" id="loopmedia" checked="false">');
    let new_loop_label = $('<label>Loop Media</label>');
    new_loop_label.append(new_loop);

    let new_videnc_mp4 = $('<input type="radio" value="mp4" name="videnctype" checked="true">');
    let new_videnc_mp4_label = $('<label>MP4</label>');
    new_videnc_mp4_label.append(new_videnc_mp4);
    let new_videnc_aud = $('<input type="radio" value="audio" name="videnctype">');
    let new_videnc_aud_label = $('<label>Audio</label>');
    new_videnc_aud_label.append(new_videnc_aud);


    this.add_entry_button = $( '<input type="button" value="Add Entry">' )
        .click(function() {
            // Get radio button value
            const typeset = $('input[name="thingtype"]:checked').val();
            const videnctype = $('input[name="videnctype"]:checked').val();
            let result = false;
            if( typeset == "sublist" ) {
                // create the sublist and add it to the common data store of the list
                result = channellist.add_sublist(new_name.val(), new_imgurl.val());
            } else {
                // create the video and add it to the common data store of the list
                result = channellist.add_video(new_name.val(), new_imgurl.val(), new_vidurl.val(), videnctype);
            }
            if( result ) {
                // use the api call to store the new version of the list
                channellist.put_channel_list_to_server();
                // re-draw the list
                channellist.draw_channel_list();
            }
        });
    this.change_entry_button = $( '<input type="button" value="Change Entry">' )
        .click(function() {
            // Get radio button value
            const videnctype = $('input[name="videnctype"]:checked').val();

            const result = channellist.change_vals(new_name.val(), new_imgurl.val(), new_vidurl.val(), videnctype, new_loop.prop("checked"));
            if( result ) {
                // use the api call to store the new version of the list
                channellist.put_channel_list_to_server();
                // re-draw the list
                channellist.draw_channel_list();
            }
        });

    channel_edit_buttons.append(new_typesub_label);
    channel_edit_buttons.append(new_typevid_label);
    channel_edit_buttons.append(new_name_label);
    channel_edit_buttons.append(new_imgurl_label);
    channel_edit_buttons.append(new_vidurl_label);
    channel_edit_buttons.append(new_videnc_mp4_label);
    channel_edit_buttons.append(new_videnc_aud_label);
    channel_edit_buttons.append(new_loop_label);
    channel_edit_buttons.append(this.add_entry_button);
    channel_edit_buttons.append(this.change_entry_button);

    let recursive_render = function(entry, cur_disp_pos) {
        let ent_disp = $( '<div class="sel_list_ent"></div>' );
        ent_disp.click(function(ev) {
            channellist.set_selection(entry, ent_disp.get()[0]); 

            // Prevent the click from bubbling up to higher level divs
            if( !ev ) {
                var ev = window.event;
            }
            ev.cancelBubble = true;
            if( ev.stopPropagation ){
                ev.stopPropagation();
            }
        });
        if( entry.type == "sublist" ) {
            ent_disp.append( $("<div></div>").text(entry.name));
            let sublist_area = $('<details></details>')
                .on('toggle', function(ev) {
                    entry.expanded = ev.target.open;
                });
            if( entry.expanded == true ) {
                sublist_area[0].open = true;
            } else {
                sublist_area[0].open = false;
            }
            let sublist_summ = $("<summary>Sublist:</summary>");
            let sublist_button_div = $('<div></div>');
            let sublist_div = $('<div></div>');
            sublist_area.append(sublist_summ);
            sublist_area.append(sublist_button_div);
            sublist_area.append(sublist_div);

            // Setup the sublist buttons
            let add_sublist = $( '<input type="button" value="Add Sublist">' )
                .click(function() {
                    let content = sublist_dialog_content();
                    let add_entry_btn = $( '<input type="button" value="Add Entry">' )
                        .click(function() {
                            let new_entry = {
                                "name": $( "#pop_addname" ).val(),
                                "image": $( "#pop_imageurl" ).val(),
                                "type": "sublist",
                                "entries": [],
                            }

                            entry.entries.push(new_entry);

                            // use the api call to store the new version of the list
                            channellist.put_channel_list_to_server();
                            // re-draw the list
                            channellist.draw_channel_list();

                            close_popup(channellist.screen_props, void(0));
                        });
                    content.append(add_entry_btn);
                    display_popup(channellist.screen_props, content);
                });
            let add_media = $( '<input type="button" value="Add Media">' )
                .click(function() {
                    let content = media_dialog_content();
                    let add_entry_btn = $( '<input type="button" value="Add Entry">' )
                        .click(function() {
                            let new_entry = {
                                "name": $( "#pop_addname" ).val(),
                                "image": $( "#pop_imageurl" ).val(),
                                "videourl": $( "#pop_videourl" ).val(),
                                "videotype": $( 'input[name="pop_videnctype"]:checked' ).val(),
                                "loop": $( "#pop_loopmedia" ).prop("checked"),
                                "type": "video",
                            }

                            entry.entries.push(new_entry);

                            // use the api call to store the new version of the list
                            channellist.put_channel_list_to_server();
                            // re-draw the list
                            channellist.draw_channel_list();

                            close_popup(channellist.screen_props, void(0));
                        });
                    content.append(add_entry_btn);
                    display_popup(channellist.screen_props, content);
                });
            let mod_sublist = $( '<input type="button" value="Modify Sublist">' )
                .click(function() {
                    let content = sublist_dialog_content(entry);
                    let mod_entry_btn = $( '<input type="button" value="Modify Entry">' )
                        .click(function() {
                            entry.name = $( "#pop_addname" ).val();
                            entry.image = $( "#pop_imageurl" ).val();

                            // use the api call to store the new version of the list
                            channellist.put_channel_list_to_server();
                            // re-draw the list
                            channellist.draw_channel_list();

                            close_popup(channellist.screen_props, void(0));
                        });
                    content.append(mod_entry_btn);
                    display_popup(channellist.screen_props, content);
                });
            sublist_button_div.append(add_sublist);
            sublist_button_div.append(add_media);
            sublist_button_div.append(mod_sublist);

            // Setup the sublist entries
            entry.entries.forEach(function (subentry) {
                recursive_render(subentry, sublist_div);
            });

            // TODO - display more
            ent_disp.append(sublist_area);
        } else if( entry.type = "video" ) {
            ent_disp.append( $("<div></div>").text(entry.name));
            let video_area = $("<details></details>")
                .on('toggle', function(ev) {
                    entry.expanded = ev.target.open;
                });
            if( entry.expanded == true ) {
                video_area[0].open = true;
            } else {
                video_area[0].open = false;
            }
            let video_div = $('<div></div>');
            let video_summ = $("<summary>Media Details:</summary>");
            let video_button_div = $('<div></div>');

            let mod_media = $( '<input type="button" value="Modify Media">' )
                .click(function() {
                    let content = media_dialog_content(entry);
                    let mod_entry_btn = $( '<input type="button" value="Mod Entry">' )
                        .click(function() {
                            entry.name = $( "#pop_addname" ).val();
                            entry.image = $( "#pop_imageurl" ).val();
                            entry.videourl = $( "#pop_videourl" ).val();
                            entry.videotype = $( 'input[name="pop_videnctype"]:checked' ).val();
                            entry.loop = $( "#pop_loopmedia" ).prop("checked");

                            // use the api call to store the new version of the list
                            channellist.put_channel_list_to_server();
                            // re-draw the list
                            channellist.draw_channel_list();

                            close_popup(channellist.screen_props, void(0));
                        });
                    content.append(mod_entry_btn);
                    display_popup(channellist.screen_props, content);
                });
            video_button_div.append(mod_media);

            video_area.append(video_summ);
            video_area.append(video_button_div);
            video_area.append(video_div);
            // TODO - display mor
            ent_disp.append(video_area);
        }
        cur_disp_pos.append(ent_disp);
        return ent_disp.get()[0];
    };

    let top_level_elem = recursive_render(this.channel_list, channel_edit_list);
    this.set_selection(this.channel_list, top_level_elem); 

    channellist.channel_list_edit_area.children().detach();
    channellist.channel_list_edit_area.append(channel_edit_list);
    channellist.channel_list_button_area.children().detach();
    channellist.channel_list_button_area.append(channel_edit_buttons);
}

ChannelList.prototype.set_selection = function (entry, domelem) {
    this.currently_selected = entry;
    $(".selected").removeClass("selected");
    domelem.classList.add("selected");

    $('input[name="thingtype"][value="' + entry.type + '"]').prop("checked", true);
    $('input[name="videnctype"][value="' + entry.videotype + '"]').prop("checked", true);
    $("#medianame").val(entry.name);
    $("#imageurl").val(entry.image);
    $("#mediaurl").val(entry.videourl);
    $("#loopmedia").prop("checked", (entry.loop === undefined) ? false : entry.loop);

    if( entry.type == "sublist" ) {
        this.add_entry_button.removeClass("notdisplayed");
        this.change_entry_button.removeClass("notdisplayed");
    } else if( entry.type == "video" ) {
        this.add_entry_button.addClass("notdisplayed");
        this.change_entry_button.removeClass("notdisplayed");
    }
}

ChannelList.prototype.change_vals = function(name, imgurl, vidurl, videnctype, loop) {
    let entry = {};
    if( this.currently_selected.type == "sublist" ) {
        this.currently_selected.name = name;
        this.currently_selected.image = imgurl;
    } else if( this.currently_selected.type == "video" ) {
        this.currently_selected.name = name;
        this.currently_selected.image = imgurl;
        this.currently_selected.videourl = vidurl;
        this.currently_selected.videotype = videnctype;
        this.currently_selected.loop = loop;
    }
    return true;
}

ChannelList.prototype.add_sublist = function (sublist_name, imgurl) {
    if( this.currently_selected.type != "sublist" ) {
        console.error("Cannot add entries to non-sublists");
        return false;
    }
    let entry = {
        "name": sublist_name,
        "image": imgurl,
        "type": "sublist",
        "entries": [],
    };
    this.currently_selected.entries.push(entry);

    return true;
}

ChannelList.prototype.add_video = function (video_name, imgurl, vidurl, videnctype) {
    if( this.currently_selected.type != "sublist" ) {
        console.error("Cannot add entries to non-sublists");
        return false;
    }
    let entry = {
        "name": video_name,
        "image": imgurl,
        "type": "video",
        "videourl": vidurl,
        "videotype": videnctype,
    };
    this.currently_selected.entries.push(entry);

    return true;
}

function validate_session_or_login_screen(screen_props) {
    $.ajax( get_api_url("validate_session_fe"),
        get_api_properties({"method": "GET"})
    ).done( function() {
        // Do nothing - we're still valid
    }).fail( function() {
        let login_screen = new LoginScreen();
        login_screen.draw(screen_props);
    });
}

function get_api_url(tail) {
    let API_ROOT = "https://api.runningstream.cc/api/v1/";
    if( document.location.host.includes("192.168.7.31") ) {
        //const API_ROOT = "http://localhost:3031/api/v1/";
        API_ROOT = "http://192.168.7.31:3031/api/v1/";
    }

    return API_ROOT + tail;
}

function get_api_properties(addtl) {
    return Object.assign({
        "crossDomain": true,
        "xhrFields": {
            "withCredentials": true,
        },
    }, addtl);
}

function display_popup(screen_props, content, after_close = function(){}) {
    screen_props.pops_back.removeClass("notdisplayed");
    let close_pop = function(ev) {
        if( screen_props.pops_back.toArray().includes(ev.target) ||
                ev.target.classList.toString().includes("close_button") ) {
            close_popup(screen_props, after_close);
            return false;
        }
        return true;
    }

    screen_props.pops_back.click( close_pop );
    screen_props.pops_back.find(".close_button").click( close_pop );

    screen_props.pops_area.empty();
    screen_props.pops_area.append(content);
}

function close_popup(screen_props, after_close = function(){}) {
    screen_props.pops_back.addClass("notdisplayed");
    after_close();
}
