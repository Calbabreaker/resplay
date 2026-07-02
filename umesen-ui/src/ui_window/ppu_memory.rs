use umesen_core::ppu::{
    NAMETABLE_SIZE_X, NAMETABLE_SIZE_Y, PATTERN_TILE_COUNT, TILE_SIZE, VramRegister,
    add_bit_planes, get_pattern_tile_addresses,
};

#[derive(Clone, Copy, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize, Debug)]
enum Tab {
    #[default]
    Palettes,
    PatternTables,
    Nametables,
    Sprites,
}

impl crate::egui_util::UiList for Tab {
    fn pretty_name(&self) -> &'static str {
        match self {
            Self::Palettes => "Palettes",
            Self::PatternTables => "Pattern Tables",
            Self::Nametables => "Nametables",
            Self::Sprites => "Sprites",
        }
    }

    const LIST: &[Self] = &[
        Self::Sprites,
        Self::Palettes,
        Self::Nametables,
        Self::PatternTables,
    ];
}

pub fn show(ui: &mut egui::Ui, state: &mut crate::State) {
    let tab_open = crate::egui_util::ui_list_tab_group::<Tab>(ui);
    ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

    match tab_open {
        Tab::Palettes => {
            for row in 0..2 {
                ui.label(if row == 0 { "Background:" } else { "Sprite:" });
                ui.horizontal(|ui| {
                    for col in 0..4 {
                        show_pallete(ui, state.emu.ppu(), row * 4 + col);
                    }
                });
            }
        }
        Tab::PatternTables => {
            ui.horizontal(|ui| {
                show_pattern_table::<0>(ui, state);
                show_pattern_table::<1>(ui, state);
            });
        }
        Tab::Nametables => {
            ui.horizontal(|ui| {
                show_nametables(ui, state);
                show_selected_nametable_info(ui, &mut state.emu);
            });
        }
        Tab::Sprites => {
            ui.horizontal(|ui| {
                show_oam_grid(ui, state);
                show_selected_oam_info(ui, state.emu.ppu());
            });
        }
    }
}

fn show_pallete(ui: &mut egui::Ui, ppu: &umesen_core::Ppu, palette_id: u8) {
    let pixel_size = egui::Vec2::splat(15.);
    let (response, painter) =
        ui.allocate_painter(pixel_size * egui::vec2(4., 1.), egui::Sense::hover());
    let mut rect = response.rect;
    for color in ppu.get_palette_colors(palette_id) {
        painter.rect_filled(
            egui::Rect::from_min_size(rect.min, pixel_size),
            0.,
            egui::Color32::from_rgb(color[0], color[1], color[2]),
        );
        rect.min.x += pixel_size.x;
    }
    response.on_hover_text(format!("ID: {}", palette_id));
}

fn show_pattern_table<const TABLE_NUMBER: u16>(ui: &mut egui::Ui, state: &mut crate::State) {
    let tile_offset = TABLE_NUMBER * PATTERN_TILE_COUNT;
    let get_tile_info_fn = |tile_index, _| {
        let tile_index = tile_index as u16;
        let palette = [[0, 0, 0], [85, 85, 85], [170, 170, 170], [255, 255, 255]];
        (tile_index + tile_offset, palette)
    };

    let config = UiPatternTilesConfig {
        name: format!("pattern{TABLE_NUMBER}"),
        tile_count: [16, 16],
        image_scale: 3.,
    };
    ui.vertical(|ui| {
        show_pattern_tiles(ui, state, &config, get_tile_info_fn);
        let id = egui::Id::new(config.name);
        if let Some(i) = ui.memory_mut(|m| m.data.get_persisted::<usize>(id)) {
            ui.label(format!("Address: ${:03x}0", (i as u16 + tile_offset)));
        }
    });
}

fn get_nametable_info(ppu: &umesen_core::Ppu, tile_index: u16) -> (u16, u8, VramRegister) {
    let mut register = VramRegister::default();
    register.set(VramRegister::NAMETABLE_X, tile_index / NAMETABLE_SIZE_X % 2);
    register.set(
        VramRegister::NAMETABLE_Y,
        tile_index / (NAMETABLE_SIZE_X * 2 * NAMETABLE_SIZE_Y),
    );
    register.set(VramRegister::COARSE_X, tile_index % NAMETABLE_SIZE_X);
    register.set(
        VramRegister::COARSE_Y,
        tile_index / (NAMETABLE_SIZE_X * 2) % NAMETABLE_SIZE_Y,
    );

    let tile_number = ppu.registers.bus.peek_read(register.nametable_address()) as u16;
    let tile_attribute = ppu.registers.bus.peek_read(register.attribute_address());
    (
        tile_number + ppu.registers.control.background_table_offset(),
        register.palette_id(tile_attribute),
        register,
    )
}

fn show_nametables(ui: &mut egui::Ui, state: &mut crate::State) {
    let config = UiPatternTilesConfig {
        name: "nametable".to_string(),
        tile_count: [NAMETABLE_SIZE_X as usize * 2, NAMETABLE_SIZE_Y as usize * 2],
        image_scale: 1.,
    };
    let get_tile_info_fn = |tile_index, ppu: &umesen_core::Ppu| {
        let (tile_number, palette_id, _) = get_nametable_info(ppu, tile_index as u16);
        (tile_number, ppu.get_palette_colors(palette_id))
    };
    let response = show_pattern_tiles(ui, state, &config, get_tile_info_fn);
    let t = state.emu.ppu().registers.t;
    let tile_x =
        t.get(VramRegister::COARSE_X) + t.get(VramRegister::NAMETABLE_X) * NAMETABLE_SIZE_X;
    let tile_y =
        t.get(VramRegister::COARSE_Y) + t.get(VramRegister::NAMETABLE_Y) * NAMETABLE_SIZE_Y;
    crate::egui_util::draw_rect_wrapped(
        ui,
        get_screen_rect(
            tile_x * TILE_SIZE as u16 + state.emu.ppu().registers.fine_x as u16,
            tile_y * TILE_SIZE as u16 + t.get(VramRegister::FINE_Y),
            NAMETABLE_SIZE_X,
            NAMETABLE_SIZE_Y,
            config.image_scale,
            response.rect,
        ),
        response.rect,
        egui::Color32::LIGHT_BLUE.gamma_multiply(0.5),
    );
}

fn show_selected_nametable_info(ui: &mut egui::Ui, emu: &mut umesen_core::Emulator) {
    ui.vertical(|ui| {
        if let Some(i) = ui.memory_mut(|m| m.data.get_persisted::<usize>("nametable".into())) {
            let mirroring = emu.cartridge().map(|c| c.mirroring()).unwrap_or_default();
            ui.label(format!("Mirroring: {mirroring:?}"));

            let (tile_number, palette_id, register) = get_nametable_info(emu.ppu(), i as u16);
            ui.label(format!("Address: ${:04x}", register.nametable_address()));
            ui.label(format!("Attribute: ${:04x}", register.attribute_address()));
            ui.label(format!("Tile: ${:03x}0", tile_number));
            ui.horizontal(|ui| {
                ui.label("Palette:");
                show_pallete(ui, emu.ppu(), palette_id);
            });
        }
    });
}

fn show_selected_oam_info(ui: &mut egui::Ui, ppu: &umesen_core::Ppu) {
    ui.vertical(|ui| {
        if let Some(i) = ui.memory_mut(|m| m.data.get_persisted("oam_grid".into())) {
            let mut sprite = ppu.registers.get_oam_sprite(i, 0).unwrap();
            ui.label(format!("Index: {i}"));
            ui.label(format!("Position (x,y): {}, {}", sprite.x, sprite.y));
            ui.label(format!(
                "Tile address: ${:03x}0",
                sprite.tile_number(&ppu.registers)
            ));

            ui.horizontal(|ui| {
                ui.label("Pallete:");
                show_pallete(ui, ppu, sprite.attributes.palette());
            });
            crate::egui_util::show_flags(ui, &mut sprite.attributes);
        }
    });
}

fn show_oam_grid(ui: &mut egui::Ui, state: &mut crate::State) {
    let get_tile_info_fn = |tile_index, ppu: &umesen_core::Ppu| {
        let sprite = ppu.registers.get_oam_sprite(tile_index, 0).unwrap();
        (
            sprite.tile_number(&ppu.registers),
            ppu.get_palette_colors(sprite.attributes.palette() + 4),
        )
    };

    let config = UiPatternTilesConfig {
        name: "oam_grid".to_string(),
        tile_count: [8, 8],
        image_scale: 4.,
    };
    show_pattern_tiles(ui, state, &config, get_tile_info_fn);
}

struct UiPatternTilesConfig {
    name: String,
    tile_count: [usize; 2],
    image_scale: f32,
}

/// Show a grid of ppu pattern table tiles
/// get_tile_info_fn is ran for each tile in the grid and should return a tuple of
/// (patern tile number, pallete id) based on the input tile index
/// If a user clicked on a tile that tile will be highligted and the tile index will be stored in
/// egui persistent memory with the id being the name
fn show_pattern_tiles<'a>(
    ui: &mut egui::Ui,
    state: &'a mut crate::State,
    config: &UiPatternTilesConfig,
    get_tile_info_fn: impl Fn(usize, &'a umesen_core::Ppu) -> (u16, [[u8; 3]; 4]),
) -> egui::Response {
    let [tile_count_x, tile_count_y] = config.tile_count;
    let id = egui::Id::new(&config.name);
    let image_size = [tile_count_x * TILE_SIZE, tile_count_y * TILE_SIZE];
    let texture = state.texture_map.get(config.name.clone(), image_size);
    let ppu = state.emu.ppu();

    let mut pixels = vec![egui::Color32::BLACK; image_size[0] * image_size[1]];
    for tile_y in 0..tile_count_y {
        for tile_x in 0..tile_count_x {
            let tile_index = (tile_y * tile_count_x) + tile_x;
            let (tile_number, palette) = get_tile_info_fn(tile_index, ppu);
            for y in 0..TILE_SIZE {
                let (lsb_address, msb_address) = get_pattern_tile_addresses(tile_number, y as u16);

                for x in 0..TILE_SIZE {
                    let pixel_index = add_bit_planes(
                        ppu.registers.bus.peek_read(lsb_address),
                        ppu.registers.bus.peek_read(msb_address),
                        0b1000_0000 >> x,
                    );
                    let pixel_x = tile_x * TILE_SIZE + x;
                    let pixel_y = tile_y * TILE_SIZE + y;
                    let c = palette[pixel_index as usize];
                    pixels[pixel_y * image_size[0] + pixel_x] =
                        egui::Color32::from_rgb(c[0], c[1], c[2]);
                }
            }
        }
    }
    texture.update_pixels(pixels);

    let image = texture
        .image(ui)
        .sense(egui::Sense::CLICK)
        .fit_to_original_size(config.image_scale);
    let response = ui
        .add(image)
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.clicked() {
        let pos = ui.input(|i| i.pointer.interact_pos()).unwrap_or_default() - response.rect.min;
        let tile_pos = pos / (TILE_SIZE as f32 * config.image_scale);
        let tile_index = (tile_pos.y as usize * tile_count_x) + tile_pos.x as usize;
        if tile_index < tile_count_x * tile_count_y {
            ui.memory_mut(|m| m.data.insert_persisted(id, tile_index))
        }
    }

    if let Some(i) = ui.memory_mut(|m| m.data.get_persisted::<usize>(id)) {
        let x = (i % tile_count_x) * TILE_SIZE;
        let y = (i / tile_count_x) * TILE_SIZE;
        let rect = get_screen_rect(x as u16, y as u16, 1, 1, config.image_scale, response.rect);
        ui.painter()
            .rect_stroke(rect, 0., (1., egui::Color32::RED), egui::StrokeKind::Inside);
    }
    response
}

fn get_screen_rect(
    x: u16,
    y: u16,
    tile_count_x: u16,
    tile_count_y: u16,
    image_scale: f32,
    image_rect: egui::Rect,
) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(x as f32, y as f32) * image_scale + image_rect.min.to_vec2(),
        egui::vec2(tile_count_x as f32, tile_count_y as f32) * (TILE_SIZE as f32 * image_scale),
    )
}
