//! Native pgvector input and binary result codecs.
use super::{value::invalid, Vector, VectorValue};
use postgres_types::{FromSql, ToSql, Type};

type CodecError = Box<dyn std::error::Error + Send + Sync>;

impl ToSql for VectorValue {
  fn to_sql(
    &self,
    _ty: &Type,
    out: &mut bytes::BytesMut,
  ) -> Result<postgres_types::IsNull, CodecError> {
    out.extend_from_slice(self.array_text().as_bytes());
    Ok(postgres_types::IsNull::No)
  }
  fn accepts(ty: &Type) -> bool {
    ty.name() == "vector"
  }
  fn encode_format(&self, _ty: &Type) -> postgres_types::Format {
    postgres_types::Format::Text
  }
  postgres_types::to_sql_checked!();
}

impl<'a, const N: usize> FromSql<'a> for Vector<N> {
  fn from_sql(_ty: &Type, raw: &'a [u8]) -> Result<Self, CodecError> {
    let header = raw
      .get(..4)
      .ok_or_else(|| invalid("missing pgvector header"))?;
    let dim_bytes: [u8; 2] = header
      .get(..2)
      .ok_or_else(|| invalid("missing dimension"))?
      .try_into()?;
    let dim = usize::from(u16::from_be_bytes(dim_bytes));
    if header.get(2..) != Some(&[0, 0][..]) || raw.len() != 4 + dim * 4 {
      return Err(invalid("invalid pgvector binary length or reserved header").into());
    }
    let payload = raw
      .get(4..)
      .ok_or_else(|| invalid("missing pgvector payload"))?;
    let mut elements = Vec::with_capacity(dim);
    for chunk in payload.chunks_exact(4) {
      elements.push(f32::from_be_bytes(chunk.try_into()?));
    }
    Ok(Self::new(&elements)?)
  }
  fn accepts(ty: &Type) -> bool {
    ty.name() == "vector"
  }
}
