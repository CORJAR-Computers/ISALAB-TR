/**
 * Limpia y formatea un número de teléfono para WhatsApp.
 * Asume prefijo +57 (Colombia) si el número tiene 10 dígitos y empieza por 3.
 */
export function formatWhatsAppNumber(phone: string | null | undefined): string {
  if (!phone) return "";
  // Quitar todo lo que no sea número
  let digits = phone.replace(/\D/g, "");
  
  if (digits.length === 10 && digits.startsWith("3")) {
    // Si es un celular colombiano sin prefijo
    digits = "57" + digits;
  }
  
  return digits;
}

/**
 * Abre la URL de WhatsApp en una nueva pestaña
 */
export function sendWhatsAppMessage(phone: string, text: string) {
  const number = formatWhatsAppNumber(phone);
  if (!number) {
    console.warn("No phone number provided to WhatsApp link");
    return;
  }
  
  const encodedText = encodeURIComponent(text);
  const url = `https://wa.me/${number}?text=${encodedText}`;
  window.open(url, "_blank");
}
