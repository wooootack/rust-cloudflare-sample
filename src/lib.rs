use rusttype::{Font, Scale};
use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
  let router = Router::new();

  router
    .post_async("/upload", |mut req, _| async move {
      let form_data = match req.form_data().await {
        Ok(form) => form,
        _ => return Response::error("不正なリクエストです", 400),
      };

      let background_image = match form_data.get("background_image") {
        Some(FormEntry::File(file)) => {
          let file = match file.type_().as_str() {
            "image/png" => file,
            "image/jpeg" => file,
            _ => return Response::error("不正なリクエストです", 400),
          };

          match image::load_from_memory(&file.bytes().await?) {
            Ok(image) => image,
            Err(e) => return Response::error(format!("画像の読み込みに失敗しました: {}", e), 500),
          }
        }
        _ => return Response::error("不正なリクエストです", 400),
      };

      let text = match form_data.get("text") {
        Some(FormEntry::Field(text)) => {
          if text.trim().is_empty() {
            return Response::error("不正なリクエストです", 400);
          } else {
            text
          }
        }
        _ => return Response::error("不正なリクエストです", 400),
      };

      let font = Vec::from(include_bytes!("../assets/Mplus1-Black.ttf") as &[u8]);
      let font = match Font::try_from_vec(font) {
        Some(font) => font,
        _ => return Response::error("フォントの読み込みに失敗しました", 500),
      };

      let scale = Scale { x: 100.0, y: 100.0 };

      let h = background_image.height();
      let w = background_image.width();

      let point = rusttype::point(0.0, font.v_metrics(scale).ascent);
      let glyphs = font
        .layout(&text, scale, point)
        .map(|g: rusttype::PositionedGlyph<'_>| g.pixel_bounding_box())
        .filter(|g| g.is_some())
        .collect::<Vec<_>>();

      let first_x = glyphs.first().unwrap().unwrap().min.x;
      let last_x = glyphs.last().unwrap().unwrap().max.x;
      let min_y = glyphs.iter().map(|g| g.unwrap().min.y).min().unwrap();
      let max_y = glyphs.iter().map(|g| g.unwrap().max.y).max().unwrap();

      let total_height = max_y - min_y;
      let total_width = last_x - first_x;

      let center_x = (w / 2) - (total_width / 2) as u32 - first_x as u32;
      let center_y = (h / 2) - (total_height / 2) as u32 - min_y as u32;

      let composite_image = imageproc::drawing::draw_text(
        &background_image,
        image::Rgba([255u8, 255u8, 255u8, 255u8]),
        center_x as i32,
        center_y as i32,
        scale,
        &font,
        &text.replace('_', " "),
      );

      let mut buffer = std::io::Cursor::new(vec![]);
      match composite_image.write_to(&mut buffer, image::ImageOutputFormat::Png) {
        Ok(_) => {}
        Err(e) => return Response::error(format!("画像の書き込みに失敗しました: {}", e), 500),
      }

      let mut headers = Headers::new();
      match headers.set("content-type", "image/png") {
        Ok(_) => {}
        Err(e) => return Response::error(format!("画像の書き込みに失敗しました: {}", e), 500),
      };

      let response = match Response::from_bytes(buffer.into_inner()) {
        Ok(response) => response,
        Err(e) => return Response::error(format!("画像の書き込みに失敗しました: {}", e), 500),
      };
      Ok(response.with_headers(headers))
    })
    .run(req, env)
    .await
}
